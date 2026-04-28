use embassy_rp::gpio::Output;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

pub(crate) static BLINK_LED: Signal<ThreadModeRawMutex, ()> = Signal::new();

#[embassy_executor::task]
pub(crate) async fn blink_led(mut pin: Output<'static>) {
    pin.set_low();
    loop {
        BLINK_LED.wait().await;
        pin.set_high();
        Timer::after_secs(1).await;
        pin.set_low();
    }
}