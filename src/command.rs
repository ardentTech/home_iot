use core::fmt::Write;
use defmt::Format;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use heapless::String;
use crate::command::Command::*;
use crate::types::{Rtc, UartMsg};
use crate::gpio::{LedCommand, GREEN_LED, RED_LED, YELLOW_LED};
use crate::rtc::{rtc_add_sec, rtc_now, rtc_sub_sec};
use crate::uart::UART_TX;

pub(crate) static EXEC_CMD: Signal<ThreadModeRawMutex, Command> = Signal::new();

pub(crate) const CMD_SIZE: usize = 3;
type Cmd = [u8; CMD_SIZE];
const ADD: Cmd = [97, 100, 100]; // "add"
const GRE: Cmd = [103, 114, 101]; // "gre"
const NOW: Cmd = [110, 111, 119]; // "now"
const RED: Cmd = [114, 101, 100]; // "red"
const SUB: Cmd = [115, 117, 98];  // "sub"
const YEL: Cmd = [121, 101, 108]; // "yel"

#[derive(Debug, Format)]
pub(crate) enum Command {
    RtcAddSec,
    RtcNow,
    RtcSubSec,
    ToggleGreenLed,
    ToggleRedLed,
    ToggleYellowLed,
}

impl TryFrom<[u8; CMD_SIZE]> for Command {
    type Error = ();

    fn try_from(value: [u8; CMD_SIZE]) -> Result<Self, Self::Error> {
        match value {
            ADD => Ok(RtcAddSec),
            GRE => Ok(ToggleGreenLed),
            NOW => Ok(RtcNow),
            RED => Ok(ToggleRedLed),
            SUB => Ok(RtcSubSec),
            YEL => Ok(ToggleYellowLed),
            _ => Err(())
        }
    }
}

pub(crate) async fn cmd_prompt() {
    let uart_sender = UART_TX.sender();
    let mut msg: UartMsg = String::new();
    core::write!(&mut msg, "\n\renter cmd: ").unwrap();
    uart_sender.send(msg).await;
}

#[embassy_executor::task]
pub(crate) async fn command_bus(rtc: &'static Rtc) {
    let uart_sender = UART_TX.sender();

    loop {
        match EXEC_CMD.wait().await {
            RtcAddSec => {
                if rtc_add_sec(rtc).await.is_err() {
                    let mut msg: UartMsg = String::new();
                    core::writeln!(&mut msg, "\n\rRtcAddSec failed\r").unwrap();
                    uart_sender.send(msg).await;
                }
            },
            RtcNow => {
                let now = rtc_now(rtc).await;
                let mut msg: UartMsg = String::new();
                core::writeln!(&mut msg, "\n\r{}\r", now).unwrap();
                uart_sender.send(msg).await;
            },
            RtcSubSec => {
                if rtc_sub_sec(rtc).await.is_err() {
                    let mut msg: UartMsg = String::new();
                    core::writeln!(&mut msg, "\n\rRtcSubSec failed\r").unwrap();
                    uart_sender.send(msg).await;
                }
            },
            ToggleGreenLed => {
                GREEN_LED.signal(LedCommand::Toggle);
            },
            ToggleRedLed => {
                RED_LED.signal(LedCommand::Toggle);
            },
            ToggleYellowLed => {
                YELLOW_LED.signal(LedCommand::Toggle);
            },
        }
        cmd_prompt().await;
    }
}