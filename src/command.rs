use core::fmt::Write;
use defmt::Format;
use embassy_time::Timer;
use heapless::String;
use crate::command::Command::*;
use crate::types::{Rtc, UartMsg};
use crate::{BLINK_LED, EXEC_CMD, UART_TX_MSG};
use crate::rtc::{rtc_add_sec, rtc_now, rtc_sub_sec};

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

pub(crate) async fn cmd_prompt() {
    let mut msg: UartMsg = String::new();
    core::write!(&mut msg, "\n\renter cmd: ").unwrap();
    UART_TX_MSG.signal(msg);
}

#[embassy_executor::task]
pub(crate) async fn command_bus(rtc: &'static Rtc) {
    loop {
        match EXEC_CMD.wait().await {
            BlinkLed => {
                BLINK_LED.signal(());
            },
            RtcAddSec => {
                if rtc_add_sec(rtc).await.is_err() {
                    let mut msg: UartMsg = String::new();
                    core::writeln!(&mut msg, "\n\rRtcAddSec failed\r").unwrap();
                    UART_TX_MSG.signal(msg);
                }
            },
            RtcNow => {
                let now = rtc_now(rtc).await;
                let mut msg: UartMsg = String::new();
                core::writeln!(&mut msg, "\n\r{}\r", now).unwrap();
                UART_TX_MSG.signal(msg);
            },
            RtcSubSec => {
                if rtc_sub_sec(rtc).await.is_err() {
                    let mut msg: UartMsg = String::new();
                    core::writeln!(&mut msg, "\n\rRtcSubSec failed\r").unwrap();
                    UART_TX_MSG.signal(msg);
                }
            }
        }
        Timer::after_millis(10).await;
        cmd_prompt().await;
    }
}