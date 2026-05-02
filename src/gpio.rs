use embassy_rp::gpio::Output;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

pub(crate) enum LedCommand {
    Pulse,
    Toggle
}

pub(crate) static GREEN_LED: Signal<ThreadModeRawMutex, LedCommand> = Signal::new();
pub(crate) static RED_LED: Signal<ThreadModeRawMutex, LedCommand> = Signal::new();
pub(crate) static YELLOW_LED: Signal<ThreadModeRawMutex, LedCommand> = Signal::new();

#[embassy_executor::task]
pub(crate) async fn green_led(mut pin: Output<'static>) {
    loop {
        match GREEN_LED.wait().await {
            LedCommand::Pulse => {
                pin.set_high();
                Timer::after_millis(100).await;
                pin.set_low()
            },
            LedCommand::Toggle => pin.toggle(),
        }
    }
}

#[embassy_executor::task]
pub(crate) async fn red_led(mut pin: Output<'static>) {
    loop {
        match RED_LED.wait().await {
            LedCommand::Pulse => {
                pin.set_high();
                Timer::after_millis(100).await;
                pin.set_low()
            },
            LedCommand::Toggle => pin.toggle(),
        }
    }
}

#[embassy_executor::task]
pub(crate) async fn yellow_led(mut pin: Output<'static>) {
    loop {
        match YELLOW_LED.wait().await {
            LedCommand::Pulse => {
                pin.set_high();
                Timer::after_millis(100).await;
                pin.set_low()
            },
            LedCommand::Toggle => pin.toggle(),
        }
    }
}