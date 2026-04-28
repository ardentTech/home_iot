use core::fmt::Write as _;
use embassy_futures::select::{select, Either};
use embassy_rp::uart::{BufferedUartRx, BufferedUartTx};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embedded_io_async::{Read, Write};
use heapless::String;
use crate::command::CMD_SIZE;
use crate::event::Event::RawCmdEntered;
use crate::event::EVENT_CHANNEL;
use crate::types::UartMsg;

pub(crate) static UART_TX: Channel<ThreadModeRawMutex, UartMsg, 8> = Channel::new();

#[embassy_executor::task]
pub(crate) async fn uart_rx(mut rx: BufferedUartRx) {
    let mut cmd_buf = [0u8; CMD_SIZE];
    let mut pointer: usize = 0;
    let event_sender = EVENT_CHANNEL.sender();
    let uart_sender = UART_TX.sender();

    loop {
        let mut buf = [0; 1];
        match rx.read_exact(&mut buf).await {
            Ok(_) => {
                if buf[0] == 13 {
                    event_sender.send(RawCmdEntered(cmd_buf)).await;
                    cmd_buf = [0u8; CMD_SIZE];
                    pointer = 0;
                } else {
                    if pointer > CMD_SIZE - 1 {
                        let mut msg: UartMsg = String::new();
                        core::writeln!(&mut msg, "invalid command length\r").unwrap();
                        uart_sender.send(msg).await;
                        cmd_buf = [0u8; CMD_SIZE];
                        pointer = 0;
                    } else {
                        let mut msg: UartMsg = String::new();
                        core::write!(&mut msg, "{}", buf[0] as char).unwrap();
                        uart_sender.send(msg).await;
                        cmd_buf[pointer] = buf[0];
                        pointer += 1;
                    }
                }
            }
            Err(_) => {}
        }
    }
}

// TODO helper (macro?) for String alloc, format and UART_TX.signal with varargs
#[embassy_executor::task]
pub(crate) async fn uart_tx(mut tx: BufferedUartTx) {
    let receiver = UART_TX.receiver();
    loop {
        let msg = receiver.receive().await;
        tx.write_all(msg.as_bytes()).await.unwrap();
    }
}