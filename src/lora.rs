use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Input, Output};
use sx127xlora::driver::{Sx127xLora, Sx127xLoraConfig};
use sx127xlora::types::{Dio0Signal, Interrupt};
use crate::{LORA_FREQUENCY_HZ, LORA_TX};
use crate::event::Event::{LoraTxDoneInterruptCleared, LoraTxDoneInterruptClearedErr, LoraTxStarted, LoraTxStartedErr};
use crate::event::EVENT_CHANNEL;
use crate::types::Spi1Bus;

#[embassy_executor::task]
pub(crate) async fn lora_modem(spi_bus: &'static Spi1Bus, cs: Output<'static>, mut dio0: Input<'static>) {
    let sender = EVENT_CHANNEL.sender();
    let spi_dev = SpiDevice::new(&spi_bus, cs);
    let mut config = Sx127xLoraConfig::default();
    config.frequency = LORA_FREQUENCY_HZ;
    let mut sx127x = Sx127xLora::new(spi_dev, config).await.expect("driver init failed :(");
    sx127x.set_temp_monitor(false).await.expect("disable temp monitor failed :(");
    sx127x.set_pa_boost(20).await.expect("set_amplifier_boost failed :(");
    sx127x.set_dio0(Dio0Signal::TxDone).await.expect("set_dio0 failed :(");

    loop {
        match select(LORA_TX.wait(), dio0.wait_for_high()).await {
            Either::First(payload) => {
                match sx127x.transmit(&payload).await {
                    Ok(_) => sender.send(LoraTxStarted).await,
                    Err(_) => sender.send(LoraTxStartedErr).await,
                }
            },
            Either::Second(_) => {
                match sx127x.clear_interrupt(Interrupt::TxDone).await {
                    Ok(_) => sender.send(LoraTxDoneInterruptCleared).await,
                    Err(_) => sender.send(LoraTxDoneInterruptClearedErr).await,
                }
            }
        }
    }
}