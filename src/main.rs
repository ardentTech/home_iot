#![no_std]
#![no_main]

#[allow(unused_imports)]
use defmt::*;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{Config, I2c, InterruptHandler};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, I2C0};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{Async, Spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use nxp_pcf8523::Pcf8523;
use nxp_pcf8523::typedefs::Pcf8523T;
use static_cell::StaticCell;
use sx127x_lora::driver::{Sx127xLora, Sx127xLoraConfig};
use sx127x_lora::types::SpreadingFactor;

type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, Async>>;

const LORA_FREQUENCY_HZ: u32 = 915_000_000;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>;
    I2C0_IRQ => InterruptHandler<I2C0>;
});

#[embassy_executor::task]
async fn led_task(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn lora_tx(spi_bus: &'static Spi1Bus, cs: Output<'static>) {
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
        Timer::after_secs(3).await;
        info!("lora_tx looping around...");
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
    static SPI_BUS: StaticCell<Spi1Bus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(spi));

    // i2c0
    // let sda = p.PIN_16;
    // let scl = p.PIN_17;
    // let mut config = Config::default();
    // config.frequency = 1_000_000;
    // let i2c_bus = I2c::new_async(p.I2C0, scl, sda, Irqs, config);

    // rtc
    //let mut pcf8523 = Pcf8523::new(i2c_bus, Pcf8523T {}).await.unwrap();
    //let mut now = pcf8523.now().await.unwrap();

    spawner.spawn(lora_tx(spi_bus, Output::new(p.PIN_13, Level::High)).unwrap());
    spawner.spawn(led_task(Output::new(p.PIN_20, Level::Low)).unwrap());
}
