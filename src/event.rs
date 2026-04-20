pub(crate) enum Event {
    PressureRead(honeywell_mpr::Reading),
    LoraTxDone,
    LoraTxNoRetriesLeft,
    RtcSecondAlarm
}