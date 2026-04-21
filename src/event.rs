pub(crate) enum Event {
    PressureRead(honeywell_mpr::Reading),
    PressureReadErr,
    LoraTxDoneInterruptCleared,
    LoraTxDoneInterruptClearedErr,
    LoraTxDone,
    LoraTxStarted,
    RtcAlarm,
}