use defmt::{Format, Formatter, write};
use packed_struct::derive::PackedStruct;
use packed_struct::PackedStruct;
use crate::types::LoraBuffer;

#[derive(PackedStruct, Clone, Copy, Debug)]
#[packed_struct(endian = "lsb")]
pub(crate) struct EnvReading {
    #[packed_field()]
    psi: u8 // TODO should be able to pack this into... 5 bits?
}

// TODO timestamp
impl EnvReading {
    pub(crate) fn new(mpr_reading: honeywell_mpr::Reading) -> Self {
        Self { psi: mpr_reading.psi() as u8 }
    }
}

impl Format for EnvReading {
    fn format(&self, fmt: Formatter) {
        write!(fmt, "{}psi", self.psi);
    }
}

impl Into<LoraBuffer> for EnvReading {
    fn into(self) -> LoraBuffer {
        let payload: [u8; 1] = self.pack().unwrap();
        let mut buffer = [0; 128];
        for (i, b) in payload.iter().enumerate() {
            buffer[i] = *b;
        }
        buffer
    }
}