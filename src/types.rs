use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_rp::i2c::I2c;
use embassy_rp::peripherals::{I2C0, SPI1};
use embassy_rp::spi::Spi;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use nxp_pcf8523::Pcf8523;
use nxp_pcf8523::typedefs::Pcf8523T;

pub(crate) type I2c0Bus = Mutex<NoopRawMutex, I2c<'static, I2C0, embassy_rp::i2c::Async>>;
pub(crate) type LoraBuffer = [u8; 128];
pub(crate) type Rtc = Mutex<NoopRawMutex, Pcf8523<I2cDevice<'static, NoopRawMutex, I2c<'static, I2C0, embassy_rp::i2c::Async>>, Pcf8523T>>;
pub(crate) type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, embassy_rp::spi::Async>>;