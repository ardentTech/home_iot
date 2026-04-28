use defmt::{Format, Formatter, write};
use embassy_futures::join::join;
use packed_struct::derive::PackedStruct;
use packed_struct::PackedStruct;
use crate::RTC_ALARM;
use crate::event::Event::EnvReadingTaken;
use crate::event::EVENT_CHANNEL;
use crate::rtc::rtc_now;
use crate::sensors::{read_aq_sensor, read_pressure_sensor};
use crate::types::{I2c0Bus, LoraBuffer, Rtc};

#[derive(Default)]
pub(crate) struct EnvReadingBuilder {
    air_pressure: Option<u8>,
    pm1: Option<u16>,
    pm2_5: Option<u16>,
    pm10: Option<u16>,
    timestamp: u32
}
impl EnvReadingBuilder {
    fn new(timestamp: u32) -> Self {
        Self {
            air_pressure: None,
            pm1: None,
            pm2_5: None,
            pm10: None,
            timestamp
        }
    }

    pub fn air_pressure(&mut self, psi: u8) {
        self.air_pressure = Some(psi);
    }

    pub fn pm1(&mut self, pm: u16) {
        self.pm1 = Some(pm);
    }

    pub fn pm2_5(&mut self, pm: u16) {
        self.pm2_5 = Some(pm);
    }

    pub fn pm10(&mut self, pm: u16) {
        self.pm10 = Some(pm);
    }

    pub fn build(self) -> EnvReading {
        EnvReading {
            air_pressure: self.air_pressure.unwrap_or(255),
            pm1: self.pm1.unwrap_or(65535),
            pm2_5: self.pm2_5.unwrap_or(65535),
            pm10: self.pm10.unwrap_or(65535),
            timestamp: self.timestamp
        }
    }
}

#[derive(PackedStruct, Clone, Copy, Debug)]
#[packed_struct(endian = "lsb")]
pub(crate) struct EnvReading { // TODO explore bit packing opportunities
    air_pressure: u8,
    pm1: u16,
    pm2_5: u16,
    pm10: u16,
    timestamp: u32
}

impl EnvReading {
    pub(crate) fn builder(timestamp: u32) -> EnvReadingBuilder {
        EnvReadingBuilder::new(timestamp)
    }
}

impl Format for EnvReading {
    fn format(&self, fmt: Formatter) {
        write!(fmt, "air pressure: {}psi, pm1: {}, pm2.5: {}, pm10: {}", self.air_pressure, self.pm1, self.pm2_5, self.pm10);
    }
}

impl Into<LoraBuffer> for EnvReading {
    fn into(self) -> LoraBuffer {
        let payload: [u8; 11] = self.pack().unwrap();
        let mut buffer = [0; 128];
        for (i, b) in payload.iter().enumerate() {
            buffer[i] = *b;
        }
        buffer
    }
}

#[embassy_executor::task]
pub(crate) async fn env_reading_task(i2c_bus: &'static I2c0Bus, rtc: &'static Rtc) {
    let sender = EVENT_CHANNEL.sender();

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
        sender.send(EnvReadingTaken(builder.build())).await;
    }
}