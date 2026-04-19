#![no_std]
#![no_main]

mod command;
mod event;

use core::sync::atomic::{AtomicBool, Ordering};
#[allow(unused_imports)]
use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{Config, I2c, InterruptHandler};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, DMA_CH2, I2C0, UART0, UART1};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{Async, Spi};
use embassy_rp::uart::UartRx;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::channel;
use embassy_sync::channel::{Channel, Receiver};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use honeywell_mpr::{Mpr, MprConfig, TransferFunction};
use nxp_pcf8523::Pcf8523;
use nxp_pcf8523::typedefs::Pcf8523T;
use static_cell::StaticCell;
use sx127x_lora::driver::{Sx127xLora, Sx127xLoraConfig};
use sx127x_lora::types::SpreadingFactor;
use crate::event::Event;

type I2c0Bus = Mutex<NoopRawMutex, I2c<'static, I2C0, embassy_rp::i2c::Async>>;
type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, Async>>;

const LORA_FREQUENCY_HZ: u32 = 915_000_000;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>, embassy_rp::dma::InterruptHandler<DMA_CH2>;
    I2C0_IRQ => InterruptHandler<I2C0>;
    UART0_IRQ => embassy_rp::uart::InterruptHandler<UART0>;
});

/// Channel for events from worker tasks to the orchestrator
static EVENT_CHANNEL: channel::Channel<CriticalSectionRawMutex, Event, 10> = channel::Channel::new();
static LED_TOGGLE: AtomicBool = AtomicBool::new(false);

#[embassy_executor::task]
async fn uart_rx_task(mut rx: UartRx<'static, embassy_rp::uart::Async>) {
    loop {
        // read a total of 4 transmissions (32 / 8) and then print the result
        let mut buf = [0; 32];
        rx.read(&mut buf).await.unwrap();
        info!("RX {:?}", buf);
        // TODO parse cmd
    }
}

#[embassy_executor::task]
async fn led_task(mut led: Output<'static>) {
    info!("led_task");
    loop {
        // TODO could use compare_exchange?
        if LED_TOGGLE.load(Ordering::Relaxed) {
            led.toggle();
            LED_TOGGLE.store(false, Ordering::Relaxed);
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
async fn lora_task(spi_bus: &'static Spi1Bus, cs: Output<'static>) {
    info!("lora_tx");
    let spi_dev = SpiDevice::new(&spi_bus, cs);
    let mut config = Sx127xLoraConfig::default();
    config.frequency = LORA_FREQUENCY_HZ;
    config.spreading_factor = SpreadingFactor::Sf12;
    let mut sx127x = Sx127xLora::new(spi_dev, config).await.expect("driver init failed :(");
    sx127x.set_temp_monitor(false).await.expect("disable temp monitor failed :(");
    // symbol duration (~33ms) is > 16ms so enable low data rate optimize
    sx127x.set_low_data_rate_optimize(true).await.expect("set_low_data_rate_optimize failed :(");
    sx127x.set_pa_boost(20).await.expect("set_amplifier_boost failed :(");

    loop {
        sx127x.transmit("howdy".as_bytes()).await.expect("transmit failed :(");
        LED_TOGGLE.store(true, Ordering::Relaxed);
        Timer::after_secs(3).await;
        info!("lora_tx looping around...");
    }
}

#[embassy_executor::task]
async fn orchestrator_task(_spawner: Spawner) {
    let receiver = EVENT_CHANNEL.receiver();
    loop {
        // Do nothing until we receive any event
        let event = receiver.receive().await;
        match event {
            Event::PressureSensorRead(reading) => info!(
                    "bar: {}, inHg: {}, mmHg: {}, kPa: {}, psi: {}",
                    reading.bar(),
                    reading.inhg(),
                    reading.mmhg(),
                    reading.kpa(),
                    reading.psi()
                )
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
        Ok(reading) => sender.send(Event::PressureSensorRead(reading)).await,
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
    //config.frequency = 1_000_000; // TODO for LoRa
    let i2c = I2c::new_async(p.I2C0, scl, sda, Irqs, config);
    static I2C0_BUS: StaticCell<I2c0Bus> = StaticCell::new();
    let i2c_bus = I2C0_BUS.init(Mutex::new(i2c));

    // rtc
    //let mut pcf8523 = Pcf8523::new(i2c_bus, Pcf8523T {}).await.unwrap();
    //let mut now = pcf8523.now().await.unwrap();

    // uart
    //let uart_rx = UartRx::new(p.UART0, p.PIN_1, Irqs, p.DMA_CH2, embassy_rp::uart::Config::default());

    spawner.spawn(orchestrator_task(spawner).unwrap());

    // spawner.spawn(lora_task(spi_bus, Output::new(p.PIN_13, Level::High)).unwrap());
    // spawner.spawn(led_task(Output::new(p.PIN_20, Level::Low)).unwrap());

    // TODO could set system state in a single place instead of multiple #[cfg(debug_assertions)] s
    //#[cfg(debug_assertions)]
    //spawner.spawn(uart_rx_task(uart_rx).unwrap());

    loop {
        spawner.spawn(pressure_sensor_task(i2c_bus).unwrap());
        Timer::after_secs(3).await;
    }
}
