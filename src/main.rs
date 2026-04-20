#![no_std]
#![no_main]

mod command;
mod event;
mod env_reading;
mod types;

use core::sync::atomic::{AtomicBool};
#[allow(unused_imports)]
use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{Config, I2c, InterruptHandler};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, DMA_CH2, I2C0, UART0, UART1};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{Async, Error, Spi};
use embassy_rp::uart::UartRx;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::channel;
use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use honeywell_mpr::{Mpr, MprConfig, TransferFunction};
use nxp_pcf8523::Pcf8523;
use nxp_pcf8523::typedefs::Pcf8523T;
use packed_struct::PackedStruct;
use static_cell::StaticCell;
use sx127x_lora::driver::{Sx127xError, Sx127xLora, Sx127xLoraConfig};
use sx127x_lora::types::{DeviceMode, Dio0Signal, Interrupt, SpreadingFactor};
use crate::env_reading::EnvReading;
use crate::event::Event;
use crate::event::Event::*;
use crate::types::LoraBuffer;

type I2c0Bus = Mutex<NoopRawMutex, I2c<'static, I2C0, embassy_rp::i2c::Async>>;
type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, Async>>;

const LORA_FREQUENCY_HZ: u32 = 915_000_000;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>, embassy_rp::dma::InterruptHandler<DMA_CH2>;
    I2C0_IRQ => InterruptHandler<I2C0>;
    UART0_IRQ => embassy_rp::uart::InterruptHandler<UART0>;
});

// TODO could group these together as... enum LoraSIgnal ?
static ENV_READING_READY: Signal<CriticalSectionRawMutex, EnvReading> = Signal::new();
static TX_DONE: Signal<CriticalSectionRawMutex, ()> = Signal::new();
/// Channel for events from worker tasks to the orchestrator
static EVENT_CHANNEL: channel::Channel<CriticalSectionRawMutex, Event, 10> = Channel::new();
static LED_TOGGLE: AtomicBool = AtomicBool::new(false);

// #[embassy_executor::task]
// async fn uart_rx_task(mut rx: UartRx<'static, embassy_rp::uart::Async>) {
//     loop {
//         // read a total of 4 transmissions (32 / 8) and then print the result
//         let mut buf = [0; 32];
//         rx.read(&mut buf).await.unwrap();
//         info!("RX {:?}", buf);
//         // TODO parse cmd
//     }
// }

// #[embassy_executor::task]
// async fn led_task(mut led: Output<'static>) {
//     info!("led_task");
//     loop {
//         // TODO could use compare_exchange?
//         if LED_TOGGLE.load(Ordering::Relaxed) {
//             led.toggle();
//             LED_TOGGLE.store(false, Ordering::Relaxed);
//         }
//         Timer::after(Duration::from_millis(100)).await;
//     }
// }

#[embassy_executor::task]
async fn lora_task(spi_bus: &'static Spi1Bus, cs: Output<'static>) {
    let spi_dev = SpiDevice::new(&spi_bus, cs);
    let mut config = Sx127xLoraConfig::default();
    config.frequency = LORA_FREQUENCY_HZ;
    config.spreading_factor = SpreadingFactor::Sf12;
    let mut sx127x = Sx127xLora::new(spi_dev, config).await.expect("driver init failed :(");
    sx127x.set_temp_monitor(false).await.expect("disable temp monitor failed :(");
    // symbol duration (~33ms) is > 16ms so enable low data rate optimize
    sx127x.set_low_data_rate_optimize(true).await.expect("set_low_data_rate_optimize failed :(");
    sx127x.set_pa_boost(20).await.expect("set_amplifier_boost failed :(");
    sx127x.set_dio0(Dio0Signal::TxDone).await.expect("set_dio0 failed :(");

    loop {
        match select(ENV_READING_READY.wait(), TX_DONE.wait()).await {
            Either::First(env_reading) => {
                lora_tx(&mut sx127x, env_reading.into(), 3).await
            },
            Either::Second(_) => {
                info!("clearing dio0");
                sx127x.clear_interrupt(Interrupt::TxDone).await.expect("clear interrupt TxDone failed :(");
            }
        }
    }
}

async fn lora_tx(
    sx127x: &mut Sx127xLora<SpiDevice<'_, NoopRawMutex, Spi<'_, SPI1, Async>, Output<'_>>>,
    buffer: LoraBuffer,
    retries: u8
) {
    let sender = EVENT_CHANNEL.sender();

    for _ in 1..=retries {
        match sx127x.transmit(&buffer).await {
            Ok(_) => return,
            Err(e) => {
                match e {
                    // TODO need better way to handle this...
                    Sx127xError::InvalidFdev => error!("Sx127xError::InvalidFdev"),
                    Sx127xError::InvalidInput => error!("Sx127xError::InvalidInput"),
                    Sx127xError::InvalidPayloadLength => error!("Sx127xError::InvalidPayloadLength"),
                    Sx127xError::InvalidPreambleLength => error!("Sx127xError::InvalidPreambleLength"),
                    Sx127xError::InvalidState => error!("Sx127xError::InvalidState"),
                    Sx127xError::InvalidSymbolTimeout => error!("Sx127xError::InvalidSymbolTimeout"),
                    Sx127xError::PacketTermination => error!("Sx127xError::PacketTermination"),
                    Sx127xError::SPI(_) => error!("Sx127xError::SPI"),
                }
                Timer::after_millis(1_000).await;
            }
        }
    }
    sender.send(LoraTxNoRetriesLeft).await;

}

#[embassy_executor::task]
async fn lora_tx_done_task(mut dio0: Input<'static>) {
    let sender = EVENT_CHANNEL.sender();

    loop {
        dio0.wait_for_high().await;
        sender.send(LoraTxDone).await;
        Timer::after_millis(100).await;
    }
}

#[embassy_executor::task]
async fn orchestrator_task(_spawner: Spawner) {
    let receiver = EVENT_CHANNEL.receiver();
    loop {
        let event = receiver.receive().await;
        match event {
            PressureRead(mpr_reading) => ENV_READING_READY.signal(EnvReading::new(mpr_reading)),
            LoraTxDone => TX_DONE.signal(()),
            LoraTxNoRetriesLeft => error!("lora tx failed :(")
        }
    }
}

#[embassy_executor::task]
async fn pressure_sensor_task(i2c_bus: &'static I2c0Bus) {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let config = MprConfig::new(0, 25, TransferFunction::C);
    let mut sensor = Mpr::new_i2c(i2c_dev, 0x18, config).unwrap();
    let sender = EVENT_CHANNEL.sender();

    if sensor.exit_standby().await.is_err() {
        error!("MPR error: exit_standby() failed :(")
    }
    Timer::after(Duration::from_millis(10)).await;
    match sensor.read().await {
        Ok(reading) => sender.send(Event::PressureRead(reading)).await,
        Err(_) => error!("MPR error: read() failed :("),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // spi1
    let miso = p.PIN_12;
    let mosi = p.PIN_11;
    let sck = p.PIN_10;
    let spi = Spi::new(p.SPI1, sck, mosi, miso, p.DMA_CH0, p.DMA_CH1, Irqs, embassy_rp::spi::Config::default());
    static SPI1_BUS: StaticCell<Spi1Bus> = StaticCell::new();
    let spi_bus = SPI1_BUS.init(Mutex::new(spi));

    // i2c0
    let sda = p.PIN_16;
    let scl = p.PIN_17;
    let mut config = Config::default();
    config.frequency = 400_000;
    let i2c = I2c::new_async(p.I2C0, scl, sda, Irqs, config);
    static I2C0_BUS: StaticCell<I2c0Bus> = StaticCell::new();
    let i2c_bus = I2C0_BUS.init(Mutex::new(i2c));

    // rtc
    //let mut pcf8523 = Pcf8523::new(i2c_bus, Pcf8523T {}).await.unwrap();
    //let mut now = pcf8523.now().await.unwrap();

    // uart
    //let uart_rx = UartRx::new(p.UART0, p.PIN_1, Irqs, p.DMA_CH2, embassy_rp::uart::Config::default());

    spawner.spawn(orchestrator_task(spawner).unwrap());

    spawner.spawn(lora_task(spi_bus, Output::new(p.PIN_13, Level::High)).unwrap());
    spawner.spawn(lora_tx_done_task(Input::new(p.PIN_15, Pull::Down)).unwrap());
    // spawner.spawn(led_task(Output::new(p.PIN_20, Level::Low)).unwrap());

    // TODO could set system state in a single place instead of multiple #[cfg(debug_assertions)] s
    //#[cfg(debug_assertions)]
    //spawner.spawn(uart_rx_task(uart_rx).unwrap());

    // this is effectively the main task
    loop {
        spawner.spawn(pressure_sensor_task(i2c_bus).unwrap());
        // TODO use rtc alarm for this
        Timer::after_secs(5).await;
    }
}
