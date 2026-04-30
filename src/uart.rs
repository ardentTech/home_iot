use core::fmt::Write as _;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::peripherals::{PIN_4, PIN_5, UART1};
use embassy_rp::uart::{BufferedUart, BufferedUartRx, BufferedUartTx};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embedded_io_async::{Read, Write};
use heapless::String;
use static_cell::StaticCell;
use crate::command::{cmd_prompt, CMD_SIZE};
use crate::event::Event::RawCmdEntered;
use crate::event::EVENT_CHANNEL;
use crate::Irqs;
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
                    event_sender.send(RawCmdEntered(cmd_buf.clone())).await;
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

#[embassy_executor::task]
pub(crate) async fn init_uart(spawner: Spawner, tx_pin: Peri<'static, PIN_4>, rx_pin: Peri<'static, PIN_5>, uart: Peri<'static, UART1>) {
    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 16])[..];
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; 16])[..];
    let uart = BufferedUart::new(uart, tx_pin, rx_pin, Irqs, tx_buf, rx_buf, embassy_rp::uart::Config::default());
    let (tx, rx) = uart.split();

    spawner.spawn(uart_rx(rx).unwrap());
    spawner.spawn(uart_tx(tx).unwrap());
    cmd_prompt().await;
}