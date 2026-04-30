use embassy_rp::gpio::Input;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use nxp_pcf8523::typedefs::{TimerA, TimerSourceClock};
use nxp_pcf8523::typedefs::TimerInterruptMode::Pulsed;
use nxp_pcf8523::typedefs::TimerMode::Countdown;
use crate::error::HomeIotError;
use crate::event::Event::RtcAlarmTriggered;
use crate::event::EVENT_CHANNEL;
use crate::types::Rtc;

pub(crate) static RTC_ALARM: Signal<ThreadModeRawMutex, ()> = Signal::new();

#[embassy_executor::task]
pub(crate) async fn rtc_alarm(rtc: &'static Rtc, mut int1_pin: Input<'static>) {
    let sender = EVENT_CHANNEL.sender();
    let cfg = TimerA::new(255, Pulsed, Countdown, TimerSourceClock::Frequency64Hz);
    {
        let mut rtc = rtc.lock().await;
        rtc.start_timer_a(&cfg).await.unwrap();
    }

    // TODO RTC now?
    loop {
        int1_pin.wait_for_falling_edge().await;
        {
            let mut rtc = rtc.lock().await;
            rtc.clear_timer_a_interrupt(&cfg).await.unwrap();
        }
        sender.send(RtcAlarmTriggered).await;
    }
}

pub(crate) async fn rtc_now(rtc: &'static Rtc) -> u32 {
    let mut rtc = rtc.lock().await;
    rtc.now().await.unwrap().timestamp()
}

pub(crate) async fn rtc_add_sec(rtc: &'static Rtc) -> Result<(), HomeIotError> {
    let mut rtc = rtc.lock().await;
    let mut now = rtc.now().await.unwrap();
    now.second = if now.second == 59 { 0 } else { now.second + 1 };
    rtc.set_datetime(now).await.map_err(|_| HomeIotError::RtcAddSec)
}

pub(crate) async fn rtc_set_day(rtc: &'static Rtc, day: u8) -> Result<(), HomeIotError> {
    let mut rtc = rtc.lock().await;
    let mut now = rtc.now().await.unwrap();
    now.day = day;
    rtc.set_datetime(now).await.map_err(|_| HomeIotError::RtcSetDay)
}

pub(crate) async fn rtc_set_hour(rtc: &'static Rtc, hour: u8) -> Result<(), HomeIotError> {
    let mut rtc = rtc.lock().await;
    let mut now = rtc.now().await.unwrap();
    now.hour = hour;
    rtc.set_datetime(now).await.map_err(|_| HomeIotError::RtcSetDay)
}

pub(crate) async fn rtc_set_min(rtc: &'static Rtc, min: u8) -> Result<(), HomeIotError> {
    let mut rtc = rtc.lock().await;
    let mut now = rtc.now().await.unwrap();
    now.minute = min;
    rtc.set_datetime(now).await.map_err(|_| HomeIotError::RtcSetDay)
}

pub(crate) async fn rtc_set_month(rtc: &'static Rtc, month: u8) -> Result<(), HomeIotError> {
    let mut rtc = rtc.lock().await;
    let mut now = rtc.now().await.unwrap();
    now.month = month;
    rtc.set_datetime(now).await.map_err(|_| HomeIotError::RtcSetDay)
}

pub(crate) async fn rtc_set_sec(rtc: &'static Rtc, sec: u8) -> Result<(), HomeIotError> {
    let mut rtc = rtc.lock().await;
    let mut now = rtc.now().await.unwrap();
    now.second = sec;
    rtc.set_datetime(now).await.map_err(|_| HomeIotError::RtcSetDay)
}

pub(crate) async fn rtc_set_year(rtc: &'static Rtc, year: u8) -> Result<(), HomeIotError> {
    let mut rtc = rtc.lock().await;
    let mut now = rtc.now().await.unwrap();
    now.year = year;
    rtc.set_datetime(now).await.map_err(|_| HomeIotError::RtcSetDay)
}

pub(crate) async fn rtc_sub_sec(rtc: &'static Rtc) -> Result<(), HomeIotError> {
    let mut rtc = rtc.lock().await;
    let mut now = rtc.now().await.unwrap();
    now.second = if now.second == 0 { 59 } else { now.second - 1 };
    rtc.set_datetime(now).await.map_err(|_| HomeIotError::RtcSubSec)
}