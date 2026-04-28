#![no_std]
#![no_main]

mod event;
mod env_reading;
mod types;
mod command;
mod error;
mod rtc;
mod sensors;
mod lora;
mod uart;

#[allow(unused_imports)]
use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{Config, I2c, InterruptHandler};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, I2C0, UART1};
use embassy_rp::spi::Spi;
use embassy_rp::uart::BufferedUart;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use nxp_pcf8523::datetime::Pcf8523DateTime;
use nxp_pcf8523::Pcf8523;
use nxp_pcf8523::typedefs::Pcf8523T;
use static_cell::StaticCell;
use crate::command::{cmd_prompt, command_bus, Command};
use crate::env_reading::{env_reading_task};
use crate::event::event_bus;
use crate::lora::lora_modem;
use crate::rtc::rtc_alarm;
use crate::types::{I2c0Bus, LoraBuffer, Rtc, Spi1Bus, UartMsg};
use crate::uart::{uart_rx, uart_tx};

const LORA_FREQUENCY_HZ: u32 = 915_000_000;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>;
    I2C0_IRQ => InterruptHandler<I2C0>;
    UART1_IRQ => embassy_rp::uart::BufferedInterruptHandler<UART1>;
});

// Signals ---------------------------------------------------------------------
static LORA_TX: Signal<ThreadModeRawMutex, LoraBuffer> = Signal::new();
static EXEC_CMD: Signal<ThreadModeRawMutex, Command> = Signal::new();
static BLINK_LED: Signal<ThreadModeRawMutex, ()> = Signal::new();
static RTC_ALARM: Signal<ThreadModeRawMutex, ()> = Signal::new();
static UART_TX_MSG: Signal<ThreadModeRawMutex, UartMsg> = Signal::new();
static UART_TX: Signal<ThreadModeRawMutex, u8> = Signal::new();

// Channels --------------------------------------------------------------------
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
    let mut pcf8523 = Pcf8523::new(I2cDevice::new(i2c_bus), Pcf8523T {}).await.unwrap();
    let dt = Pcf8523DateTime::new(0, 0, 0, 8, 19, 25).unwrap();
    pcf8523.set_datetime(dt).await.unwrap();
    static SHARED_RTC: StaticCell<Rtc> = StaticCell::new();
    let shared_rtc = SHARED_RTC.init(Mutex::new(pcf8523));

    // uart1
    let (tx_pin, rx_pin, uart) = (p.PIN_4, p.PIN_5, p.UART1);
    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 16])[..];
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; 16])[..];
    let uart = BufferedUart::new(uart, tx_pin, rx_pin, Irqs, tx_buf, rx_buf, embassy_rp::uart::Config::default());
    let (tx, rx) = uart.split();

    spawner.spawn(event_bus().unwrap());
    spawner.spawn(command_bus(shared_rtc).unwrap());
    spawner.spawn(rtc_alarm(shared_rtc, Input::new(p.PIN_8, Pull::Up)).unwrap());
    spawner.spawn(env_reading_task(i2c_bus, shared_rtc).unwrap()); // TODO does this have to be a task?
    spawner.spawn(lora_modem(spi_bus, Output::new(p.PIN_13, Level::High), Input::new(p.PIN_15, Pull::Down)).unwrap());
    spawner.spawn(blink_led(Output::new(p.PIN_20, Level::Low)).unwrap());
    spawner.spawn(uart_rx(rx).unwrap());
    spawner.spawn(uart_tx(tx).unwrap());
    cmd_prompt().await;
}