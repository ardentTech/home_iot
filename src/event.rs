pub(crate) enum Event {
    PressureSensorRead(honeywell_mpr::Reading),
    PressureSensorReadErr,
    LoraTxDoneInterruptCleared,
    LoraTxDoneInterruptClearedErr,
    LoraTxStarted,
    RtcAlarmTriggered,
}