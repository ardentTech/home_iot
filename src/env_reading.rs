use defmt::{Format, Formatter, write};
use packed_struct::derive::PackedStruct;
use packed_struct::PackedStruct;
use crate::types::LoraBuffer;

#[derive(Default)]
pub struct EnvReadingBuilder {
    pressure_psi: Option<u8>,
    timestamp: u32
}
impl EnvReadingBuilder {
    pub fn new(timestamp: u32) -> Self {
        Self {
            pressure_psi: None,
            timestamp
        }
    }

    pub fn pressure_psi(&mut self, psi: u8){
        self.pressure_psi = Some(psi);
    }

    pub fn build(self) -> EnvReading {
        EnvReading {
            pressure_psi: self.pressure_psi.unwrap_or(255),
            timestamp: self.timestamp
        }
    }
}

#[derive(PackedStruct, Clone, Copy, Debug)]
#[packed_struct(endian = "lsb")]
pub(crate) struct EnvReading {
    pub(crate) pressure_psi: u8, // TODO should be able to pack this into... 5 bits?
    pub(crate) timestamp: u32
}

impl EnvReading {
    pub(crate) fn builder() -> EnvReadingBuilder {
        EnvReadingBuilder::default()
    }
}

impl Format for EnvReading {
    fn format(&self, fmt: Formatter) {
        write!(fmt, "{}psi", self.pressure_psi);
    }
}

impl Into<LoraBuffer> for EnvReading {
    fn into(self) -> LoraBuffer {
        let payload: [u8; 5] = self.pack().unwrap();
        let mut buffer = [0; 128];
        for (i, b) in payload.iter().enumerate() {
            buffer[i] = *b;
        }
        buffer
    }
}