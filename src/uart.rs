use core::fmt::Write as _;
use embassy_futures::select::{select, Either};
use embassy_rp::uart::{BufferedUartRx, BufferedUartTx};
use embedded_io_async::{Read, Write};
use heapless::String;
use crate::{UART_TX, UART_TX_MSG};
use crate::command::CMD_SIZE;
use crate::event::Event::RawCmdEntered;
use crate::event::EVENT_CHANNEL;
use crate::types::UartMsg;

#[embassy_executor::task]
pub(crate) async fn uart_rx(mut rx: BufferedUartRx) {
    let mut cmd_buf = [0u8; CMD_SIZE];
    let mut pointer: usize = 0;
    let sender = EVENT_CHANNEL.sender();

    loop {
        let mut buf = [0; 1];
        match rx.read_exact(&mut buf).await {
            Ok(_) => {
                if buf[0] == 13 {
                    sender.send(RawCmdEntered(cmd_buf)).await;
                    cmd_buf = [0u8; CMD_SIZE];
                    pointer = 0;
                } else {
                    if pointer > CMD_SIZE - 1 {
                        let mut msg: UartMsg = String::new();
                        core::writeln!(&mut msg, "invalid command length\r").unwrap();
                        UART_TX_MSG.signal(msg);
                        cmd_buf = [0u8; CMD_SIZE];
                        pointer = 0;
                    } else {
                        UART_TX.signal(buf[0]);
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
    loop {
        match select(UART_TX.wait(), UART_TX_MSG.wait()).await {
            Either::First(byte) => {
                //debug!("UART_TX matched");
                tx.write_all(&[byte]).await.unwrap()
            },
            Either::Second(msg) => {
                //debug!("UART_TX_MSG matched: {}", &msg.as_bytes());
                tx.write_all(msg.as_bytes()).await.unwrap()
            },
        }
    }
}