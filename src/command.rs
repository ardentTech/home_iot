use defmt::Format;
use embassy_sync::pubsub::subscriber::Sub;
use crate::command::Command::*;

pub(crate) const CMD_SIZE: usize = 3;

type Cmd = [u8; CMD_SIZE];
const ADD: Cmd = [97, 100, 100]; // "add"
const LED: Cmd = [108, 101, 100]; // "led"
const NOW: Cmd = [110, 111, 119]; // "now"
const SUB: Cmd = [115, 117, 98];  // "sub"

#[derive(Debug, Format)]
pub(crate) enum Command {
    BlinkLed,
    RtcAddSec,
    RtcNow,
    RtcSubSec,
}

impl TryFrom<[u8; CMD_SIZE]> for Command {
    type Error = ();

    fn try_from(value: [u8; CMD_SIZE]) -> Result<Self, Self::Error> {
        match value {
            ADD => Ok(RtcAddSec),
            LED => Ok(BlinkLed),
            NOW => Ok(RtcNow),
            SUB => Ok(RtcSubSec),
            _ => Err(())
        }
    }
}