use core::fmt::Write;
use defmt::{debug, Format};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use heapless::{String, Vec};
use crate::command::Command::*;
use crate::types::{Rtc, UartMsg};
use crate::gpio::{LedCommand, GREEN_LED, RED_LED, YELLOW_LED};
use crate::rtc::{rtc_add_sec, rtc_now, rtc_set_day, rtc_set_hour, rtc_set_min, rtc_set_month, rtc_set_sec, rtc_set_year, rtc_sub_sec};
use crate::uart::UART_TX;

pub(crate) static EXEC_CMD: Signal<ThreadModeRawMutex, Command> = Signal::new();

pub(crate) const CMD_SIZE: usize = 17;

#[derive(Debug, Format)]
pub(crate) enum Command {
    RtcAddSec,
    RtcNow,
    RtcSetDay(u8),
    RtcSetHour(u8),
    RtcSetMin(u8),
    RtcSetMonth(u8),
    RtcSetSec(u8),
    RtcSetYear(u8),
    RtcSubSec,
    PulseGreenLed,
    PulseRedLed,
    PulseYellowLed,
}

impl TryFrom<[u8; CMD_SIZE]> for Command {
    type Error = ();
    fn try_from(raw: [u8; CMD_SIZE]) -> Result<Self, Self::Error> {
        let mut s = Vec::<u8, 32>::new();
        s.extend_from_slice(&raw).map_err(|_| ())?;
        let s: String<32> = String::from_utf8(s).map_err(|_| ())?;
        let mut iter = s.split(' ');

        if let Some(cmd) = iter.next() {
            match cmd.trim_matches(char::from(0)) {
                "green_led_pulse" => Ok(PulseGreenLed),
                "red_led_pulse" => Ok(PulseRedLed),
                "rtc_add_sec" => Ok(RtcAddSec),
                "rtc_now" => Ok(RtcNow),
                // TODO need to validate day, hour, etc.?
                "rtc_set_day" => {
                    if let Some(day) = iter.next() {
                        Ok(RtcSetDay(day.trim_matches(char::from(0)).parse::<u8>().map_err(|_| ())?))
                    } else { Err(()) }
                },
                "rtc_set_hour" => {
                    if let Some(hour) = iter.next() {
                        Ok(RtcSetHour(hour.trim_matches(char::from(0)).parse::<u8>().map_err(|_| ())?))
                    } else { Err(()) }
                },
                "rtc_set_min" => {
                    if let Some(min) = iter.next() {
                        Ok(RtcSetMin(min.trim_matches(char::from(0)).parse::<u8>().map_err(|_| ())?))
                    } else { Err(()) }
                },
                "rtc_set_month" => {
                    if let Some(month) = iter.next() {
                        Ok(RtcSetMonth(month.trim_matches(char::from(0)).parse::<u8>().map_err(|_| ())?))
                    } else { Err(()) }
                },
                "rtc_set_sec" => {
                    if let Some(sec) = iter.next() {
                        Ok(RtcSetSec(sec.trim_matches(char::from(0)).parse::<u8>().map_err(|_| ())?))
                    } else { Err(()) }
                },
                "rtc_set_year" => {
                    if let Some(year) = iter.next() {
                        Ok(RtcSetYear(year.trim_matches(char::from(0)).parse::<u8>().map_err(|_| ())?))
                    } else { Err(()) }
                },
                "rtc_sub_sec" => Ok(RtcSubSec),
                "yellow_led_pulse" => Ok(PulseYellowLed),
                _ => Err(())
            }
        } else {
            Err(())
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
                if let Err(e) = rtc_add_sec(rtc).await {
                    uart_sender.send(e.into()).await;
                }
            },
            RtcNow => {
                let now = rtc_now(rtc).await;
                let mut msg: UartMsg = String::new();
                core::writeln!(&mut msg, "\n\r{}\r", now).unwrap();
                uart_sender.send(msg).await;
            },
            RtcSetDay(day) => {
                if let Err(e) = rtc_set_day(rtc, day).await {
                    uart_sender.send(e.into()).await;
                }
            },
            RtcSetHour(hour) => {
                if let Err(e) = rtc_set_hour(rtc, hour).await {
                    uart_sender.send(e.into()).await;
                }
            },
            RtcSetMin(min) => {
                if let Err(e) = rtc_set_min(rtc, min).await {
                    uart_sender.send(e.into()).await;
                }
            },
            RtcSetMonth(month) => {
                if let Err(e) = rtc_set_month(rtc, month).await {
                    uart_sender.send(e.into()).await;
                }
            },
            RtcSetSec(sec) => {
                if let Err(e) = rtc_set_sec(rtc, sec).await {
                    uart_sender.send(e.into()).await;
                }
            },
            RtcSetYear(year) => {
                if let Err(e) = rtc_set_year(rtc, year).await {
                    uart_sender.send(e.into()).await;
                }
            },
            RtcSubSec => {
                if let Err(e) = rtc_sub_sec(rtc).await {
                    uart_sender.send(e.into()).await;
                }
            },
            PulseGreenLed => {
                GREEN_LED.signal(LedCommand::Pulse);
            },
            PulseRedLed => {
                RED_LED.signal(LedCommand::Pulse);
            },
            PulseYellowLed => {
                YELLOW_LED.signal(LedCommand::Pulse);
            },
        }
        cmd_prompt().await;
    }
}