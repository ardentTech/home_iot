#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{Config, I2c, InterruptHandler};
use embassy_rp::peripherals::I2C0;
use embassy_time::Timer;
use nxp_pcf8523::Pcf8523;
use nxp_pcf8523::typedefs::Pcf8523T;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
});


#[embassy_executor::main]
async fn main(_task_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let sda = p.PIN_16;
    let scl = p.PIN_17;
    let mut led = Output::new(p.PIN_20, Level::Low);

    let mut config = Config::default();
    config.frequency = 1_000_000;
    let i2c_bus = I2c::new_async(p.I2C0, scl, sda, Irqs, config);

    let mut pcf8523 = Pcf8523::new(i2c_bus, Pcf8523T {}).await.unwrap();
    let mut now = pcf8523.now().await.unwrap();

    loop {
        now = pcf8523.now().await.unwrap();
        info!("now: {}", now.timestamp());
        led.toggle();
        Timer::after_secs(3).await;
    }
}
