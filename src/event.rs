use core::fmt::Write;
use defmt::{debug, error};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use heapless::String;
use crate::command::{cmd_prompt, Command, CMD_SIZE};
use crate::env_reading::EnvReading;
use crate::event::Event::{EnvReadingTaken, LoraTxDoneInterruptCleared, LoraTxDoneInterruptClearedErr, LoraTxStarted, LoraTxStartedErr, RawCmdEntered, RtcAlarmTriggered};
use crate::{EXEC_CMD, LORA_TX, RTC_ALARM, UART_TX_MSG};
use crate::types::UartMsg;

pub(crate) static EVENT_CHANNEL: Channel<ThreadModeRawMutex, Event, 10> = Channel::new();

pub(crate) enum Event {
    EnvReadingTaken(EnvReading),
    LoraTxDoneInterruptCleared,
    LoraTxDoneInterruptClearedErr,
    LoraTxStarted,
    LoraTxStartedErr,
    RawCmdEntered([u8; CMD_SIZE]),
    RtcAlarmTriggered,
}

#[embassy_executor::task]
pub(crate) async fn event_bus() {
    let receiver = EVENT_CHANNEL.receiver();

    loop {
        let event = receiver.receive().await;
        match event {
            EnvReadingTaken(env_reading) => {
                LORA_TX.signal(env_reading.into());
            },
            LoraTxDoneInterruptCleared => debug!("lora tx done interrupt cleared"),
            LoraTxDoneInterruptClearedErr => error!("lora tx done interrupt cleared err :("),
            LoraTxStarted => debug!("lora tx started"),
            LoraTxStartedErr => error!("lora tx started err :("),
            RawCmdEntered(raw) => {
                match Command::try_from(raw) {
                    Ok(cmd) => EXEC_CMD.signal(cmd),
                    Err(_) => {
                        let mut msg: UartMsg = String::new();
                        core::writeln!(&mut msg, "\n\rinvalid command\r").unwrap();
                        UART_TX_MSG.signal(msg);
                        Timer::after_millis(250).await;
                        cmd_prompt().await;
                    },
                }
            },
            RtcAlarmTriggered => {
                RTC_ALARM.signal(())
            },
        }
    }
}