use defmt::error;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::{Duration, Timer};
use honeywell_mpr::{Mpr, MprConfig, TransferFunction};
use pmsa003i::Pmsa003i;
use crate::types::I2c0Bus;

pub(crate) async fn read_aq_sensor(i2c_bus: &'static I2c0Bus) -> Option<pmsa003i::Reading> {
    let i2c_dev = I2cDevice::new(i2c_bus);
    let mut aq_sensor = Pmsa003i::new(i2c_dev);

    match aq_sensor.read().await {
        Ok(reading) => Some(reading),
        Err(_) => None
    }
}

pub(crate) async fn read_pressure_sensor(i2c_bus: &'static I2c0Bus) -> Option<honeywell_mpr::Reading> {
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