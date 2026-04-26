use crate::command::CMD_SIZE;

pub(crate) enum Event {
    RawCmdEntered([u8; CMD_SIZE]),
    PressureSensorRead(honeywell_mpr::Reading),
    PressureSensorReadErr,
    LoraTxDoneInterruptCleared,
    LoraTxDoneInterruptClearedErr,
    LoraTxStarted,
    LoraTxStartedErr,
    RtcAlarmTriggered,
}