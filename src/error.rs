use core::fmt::Write;
use heapless::String;
use crate::types::UartMsg;

#[derive(Debug)]
pub(crate) enum HomeIotError {
    RtcAddSec,
    RtcSetDay,
    RtcSubSec
}

impl Into<UartMsg> for HomeIotError {
    fn into(self) -> UartMsg {
        let mut msg: UartMsg = String::new();
        core::writeln!(&mut msg, "\n\r{:?} failed\r", self).unwrap();
        msg
    }
}