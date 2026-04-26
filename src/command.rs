use defmt::Format;
use crate::command::Command::BlinkLed;

pub(crate) const CMD_SIZE: usize = 3;

#[derive(Debug, Format)]
pub(crate) enum Command {
    BlinkLed
}

impl TryFrom<[u8; CMD_SIZE]> for Command {
    type Error = ();

    fn try_from(value: [u8; CMD_SIZE]) -> Result<Self, Self::Error> {
        match value {
            [108, 101, 100] | [76, 69, 68] => Ok(BlinkLed),
            _ => Err(())
        }
    }
}