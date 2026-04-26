use crate::command::CMD_SIZE;
use crate::env_reading::EnvReading;

pub(crate) enum Event {
    EnvReadingTaken(EnvReading),
    LoraTxDoneInterruptCleared,
    LoraTxDoneInterruptClearedErr,
    LoraTxStarted,
    LoraTxStartedErr,
    RawCmdEntered([u8; CMD_SIZE]),
    RtcAlarmTriggered,
}