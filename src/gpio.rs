use embassy_rp::gpio::Output;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

pub(crate) enum Led {
    Green,
    Red,
    Yellow
}

const PULSE_DURATION_MS: u64 = 100;

pub(crate) static PULSE_LED: Signal<ThreadModeRawMutex, Led> = Signal::new();

#[embassy_executor::task]
pub(crate) async fn pulse_led_task(mut green: Output<'static>, mut red: Output<'static>, mut yellow: Output<'static>) {
    loop {
        let pin = match PULSE_LED.wait().await {
            Led::Green => &mut green,
            Led::Red => &mut red,
            Led::Yellow => &mut yellow,
        };
        pulse(pin).await;
    }
}

async fn pulse(pin: &mut Output<'static>) {
    pin.set_high();
    Timer::after_millis(PULSE_DURATION_MS).await;
    pin.set_low();
}