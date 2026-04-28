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
mod gpio;

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
use embassy_sync::mutex::Mutex;
use nxp_pcf8523::datetime::Pcf8523DateTime;
use nxp_pcf8523::Pcf8523;
use nxp_pcf8523::typedefs::Pcf8523T;
use static_cell::StaticCell;
use crate::command::command_bus;
use crate::env_reading::{env_reading_task};
use crate::event::event_bus;
use crate::gpio::blink_led;
use crate::lora::lora_modem;
use crate::rtc::rtc_alarm;
use crate::types::{I2c0Bus, Rtc, Spi1Bus};
use crate::uart::init_uart;

const LORA_FREQUENCY_HZ: u32 = 915_000_000;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>;
    I2C0_IRQ => InterruptHandler<I2C0>;
    UART1_IRQ => embassy_rp::uart::BufferedInterruptHandler<UART1>;
});

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

    spawner.spawn(event_bus().unwrap());
    spawner.spawn(command_bus(shared_rtc).unwrap());
    spawner.spawn(rtc_alarm(shared_rtc, Input::new(p.PIN_8, Pull::Up)).unwrap());
    spawner.spawn(env_reading_task(i2c_bus, shared_rtc).unwrap()); // TODO does this have to be a task?
    spawner.spawn(lora_modem(spi_bus, Output::new(p.PIN_13, Level::High), Input::new(p.PIN_15, Pull::Down)).unwrap());
    spawner.spawn(blink_led(Output::new(p.PIN_20, Level::Low)).unwrap());
    spawner.spawn(init_uart(spawner, p.PIN_4, p.PIN_5, p.UART1).unwrap());
}