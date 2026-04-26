#![no_std]
#![no_main]

mod event;
mod env_reading;
mod types;
mod command;

use circular_buffer::CircularBuffer;
#[allow(unused_imports)]
use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_rp::{bind_interrupts, uart};
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{Config, I2c, InterruptHandler};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, DMA_CH2, DMA_CH3, I2C0, UART1};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{Async, Spi};
use embassy_rp::uart::{BufferedUart, BufferedUartRx, BufferedUartTx, Error, UartRx, UartTx};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::{Channel};
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, ReadExactError, Write};
use honeywell_mpr::{Mpr, MprConfig, TransferFunction};
use nxp_pcf8523::Pcf8523;
use nxp_pcf8523::typedefs::{Pcf8523T, TimerA, TimerSourceClock};
use nxp_pcf8523::typedefs::TimerInterruptMode::Pulsed;
use nxp_pcf8523::typedefs::TimerMode::Countdown;
use pmsa003i::Pmsa003i;
use static_cell::StaticCell;
use sx127xlora::driver::{Sx127xLora, Sx127xLoraConfig};
use sx127xlora::types::{Dio0Signal, Interrupt};
use crate::command::{Command, CMD_SIZE};
use crate::env_reading::EnvReading;
use crate::event::Event;
use crate::event::Event::*;
use crate::types::LoraBuffer;

type I2c0Bus = Mutex<NoopRawMutex, I2c<'static, I2C0, embassy_rp::i2c::Async>>;
type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, Async>>;
type Rtc = Mutex<NoopRawMutex, Pcf8523<I2cDevice<'static, NoopRawMutex, I2c<'static, I2C0, embassy_rp::i2c::Async>>, Pcf8523T>>;

const LORA_FREQUENCY_HZ: u32 = 915_000_000;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>, embassy_rp::dma::InterruptHandler<DMA_CH2>, embassy_rp::dma::InterruptHandler<DMA_CH3>;
    I2C0_IRQ => InterruptHandler<I2C0>;
    UART1_IRQ => uart::BufferedInterruptHandler<UART1>;
});

// Signals ---------------------------------------------------------------------
static ENV_READING_READY: Signal<ThreadModeRawMutex, EnvReading> = Signal::new();
static EXEC_CMD: Signal<ThreadModeRawMutex, Command> = Signal::new();
static BLINK_LED: Signal<ThreadModeRawMutex, ()> = Signal::new();
static RTC_ALARM: Signal<ThreadModeRawMutex, ()> = Signal::new();
static UART_TX: Signal<ThreadModeRawMutex, &[u8]> = Signal::new();

// Channels --------------------------------------------------------------------
static EVENT_CHANNEL: Channel<ThreadModeRawMutex, Event, 10> = Channel::new();

// Buffers ---------------------------------------------------------------------
static UART_TX_BUFFER: CircularBuffer<16, u8> = CircularBuffer::new();

#[embassy_executor::task]
async fn env_reading_task(i2c_bus: &'static I2c0Bus, rtc: &'static Rtc) {
    loop {
        RTC_ALARM.wait().await;
        let (aq_res, pressure_res) = join(read_aq_sensor(i2c_bus), read_pressure_sensor(i2c_bus)).await;
        let now = rtc_now(rtc).await;
        let mut builder = EnvReading::builder(now);

        if let Some(aq) = aq_res {
            builder.pm1(aq.pm1);
            builder.pm2_5(aq.pm2_5);
            builder.pm10(aq.pm10);
        }

        if let Some(pressure) = pressure_res {
            builder.air_pressure(pressure.psi() as u8);
        }
        ENV_READING_READY.signal(builder.build())
    }
}

async fn read_aq_sensor(i2c_bus: &'static I2c0Bus) -> Option<pmsa003i::Reading> {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut aq_sensor = Pmsa003i::new(i2c_dev);

    match aq_sensor.read().await {
        Ok(reading) => Some(reading),
        Err(_) => None
    }
}

async fn read_pressure_sensor(i2c_bus: &'static I2c0Bus) -> Option<honeywell_mpr::Reading> {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let config = MprConfig::new(0, 25, TransferFunction::C);
    let mut sensor = Mpr::new_i2c(i2c_dev, 0x18, config).unwrap();

    if sensor.exit_standby().await.is_err() {
        error!("MPR error: exit_standby() failed :(");
        None
    } else {
        Timer::after(Duration::from_millis(10)).await;
        match sensor.read().await {
            Ok(reading) => Some(reading),
            Err(_) => None
        }
    }
}

async fn rtc_now(rtc: &'static Rtc) -> u32 {
    let mut rtc = rtc.lock().await;
    rtc.now().await.unwrap().timestamp()
}

#[embassy_executor::task]
async fn lora_modem(spi_bus: &'static Spi1Bus, cs: Output<'static>, mut dio0: Input<'static>) {
    let sender = EVENT_CHANNEL.sender();
    let spi_dev = SpiDevice::new(&spi_bus, cs);
    let mut config = Sx127xLoraConfig::default();
    config.frequency = LORA_FREQUENCY_HZ;
    let mut sx127x = Sx127xLora::new(spi_dev, config).await.expect("driver init failed :(");
    sx127x.set_temp_monitor(false).await.expect("disable temp monitor failed :(");
    sx127x.set_pa_boost(20).await.expect("set_amplifier_boost failed :(");
    sx127x.set_dio0(Dio0Signal::TxDone).await.expect("set_dio0 failed :(");

    loop {
        match select(ENV_READING_READY.wait(), dio0.wait_for_high()).await {
            Either::First(env_reading) => {
                debug!("env_reading: {:?}", env_reading);
                let payload: LoraBuffer = env_reading.into();
                match sx127x.transmit(&payload).await {
                    Ok(_) => sender.send(LoraTxStarted).await,
                    Err(_) => sender.send(LoraTxStartedErr).await,
                }
            },
            Either::Second(_) => {
                match sx127x.clear_interrupt(Interrupt::TxDone).await {
                    Ok(_) => sender.send(LoraTxDoneInterruptCleared).await,
                    Err(_) => sender.send(LoraTxDoneInterruptClearedErr).await,
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn event_bus() {
    let receiver = EVENT_CHANNEL.receiver();
    loop {
        let event = receiver.receive().await;
        match event {
            RawCmdEntered(raw) => {
                match Command::try_from(raw) {
                    Ok(cmd) => EXEC_CMD.signal(cmd),
                    Err(_) => UART_TX.signal("invalid cmd :(\r\n\nenter cmd: ".as_bytes()),
                }
            },
            PressureSensorRead(_) => debug!("pressure sensor read"),
            PressureSensorReadErr => error!("pressure sensor read err :("),
            LoraTxDoneInterruptCleared => debug!("lora tx done interrupt cleared"),
            LoraTxDoneInterruptClearedErr => error!("lora tx done interrupt cleared err :("),
            LoraTxStarted => debug!("lora tx started"),
            LoraTxStartedErr => error!("lora tx started err :("),
            RtcAlarmTriggered => {
                debug!("rtc alarm triggered");
                RTC_ALARM.signal(())
            },
        }
    }
}

#[embassy_executor::task]
async fn rtc_alarm(rtc: &'static Rtc, mut int1_pin: Input<'static>) {
    let sender = EVENT_CHANNEL.sender();
    let cfg = TimerA::new(255, Pulsed, Countdown, TimerSourceClock::Frequency64Hz);
    {
        let mut rtc = rtc.lock().await;
        rtc.start_timer_a(&cfg).await.unwrap();
    }

    loop {
        int1_pin.wait_for_falling_edge().await;
        {
            let mut rtc = rtc.lock().await;
            rtc.clear_timer_a_interrupt(&cfg).await.unwrap();
        }
        sender.send(RtcAlarmTriggered).await;
    }
}

#[embassy_executor::task]
async fn blink_led(mut pin: Output<'static>) {
    pin.set_low();
    loop {
        BLINK_LED.wait().await;
        pin.set_high();
        Timer::after_secs(1).await;
        pin.set_low();
    }
}

#[embassy_executor::task]
async fn command_bus() {
    loop {
        match EXEC_CMD.wait().await {
            Command::BlinkLed => {
                BLINK_LED.signal(());
                UART_TX.signal("executed BlinkLed cmd\r\n\nenter cmd: ".as_bytes());
            },
        }
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
    config.frequency = 100_000; // this is the only I2C bus speed that works with rtc, pressure and aq peripherals
    let i2c = I2c::new_async(p.I2C0, scl, sda, Irqs, config);
    static I2C0_BUS: StaticCell<I2c0Bus> = StaticCell::new();
    let i2c_bus = I2C0_BUS.init(Mutex::new(i2c));

    // rtc
    let pcf8523 = Pcf8523::new(I2cDevice::new(i2c_bus), Pcf8523T {}).await.unwrap();
    static SHARED_RTC: StaticCell<Rtc> = StaticCell::new();
    let shared_rtc = SHARED_RTC.init(Mutex::new(pcf8523));

    spawner.spawn(event_bus().unwrap());
    spawner.spawn(command_bus().unwrap());
    // spawner.spawn(alarm_task(shared_rtc, Input::new(p.PIN_8, Pull::Up)).unwrap());
    // spawner.spawn(env_reading_task(i2c_bus, shared_rtc).unwrap());
    // spawner.spawn(lora_task(spi_bus, Output::new(p.PIN_13, Level::High), Input::new(p.PIN_15, Pull::Down)).unwrap());
    spawner.spawn(blink_led(Output::new(p.PIN_20, Level::Low)).unwrap());

    let (tx_pin, rx_pin, uart) = (p.PIN_4, p.PIN_5, p.UART1);
    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 16])[..];
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; 16])[..];
    let uart = BufferedUart::new(uart, tx_pin, rx_pin, Irqs, tx_buf, rx_buf, uart::Config::default());
    let (tx, rx) = uart.split();

    spawner.spawn(uart_rx(rx).unwrap());
    spawner.spawn(uart_tx(tx).unwrap());
}

#[embassy_executor::task]
async fn uart_rx(mut rx: BufferedUartRx) {
    let mut cmd_buf = [0u8; CMD_SIZE];
    let mut pointer: usize = 0;
    let sender = EVENT_CHANNEL.sender();

    // TODO uart tx msg (e.g. "ready for cmds")
    loop {
        let mut buf = [0; 1];
        match rx.read_exact(&mut buf).await {
            Ok(_) => {
                if buf[0] == 13 {
                    sender.send(RawCmdEntered(cmd_buf)).await;
                    cmd_buf = [0u8; CMD_SIZE];
                    pointer = 0;
                } else {
                    if pointer > CMD_SIZE - 1 {
                        // TODO uart tx error msg
                        error!("cmd too long!");
                        cmd_buf = [0u8; CMD_SIZE];
                        pointer = 0;
                    } else {
                        cmd_buf[pointer] = buf[0];
                        pointer += 1;
                    }
                }
            }
            Err(_) => {}
        }
    }
}

#[embassy_executor::task]
async fn uart_tx(mut tx: BufferedUartTx) {
    tx.write_all("enter cmd: ".as_bytes()).await.unwrap();
    loop {
        let msg = UART_TX.wait().await;
        tx.write_all(msg).await.unwrap();
    }
}