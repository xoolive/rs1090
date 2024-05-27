use crate::decode::TimeSource;
use crate::prelude::*;
use crate::source::beast::DataSource;
use futures_util::pin_mut;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc;

pub async fn receiver(
    address: String,
    tx: mpsc::Sender<TimedMessage>,
    idx: usize,
) {
    match TcpStream::connect(&address).await {
        Ok(stream) => {
            let msg_stream = beast::next_msg(DataSource::Tcp(stream)).await;
            pin_mut!(msg_stream); // needed for iteration
            'receive: loop {
                while let Some(msg) = msg_stream.next().await {
                    let msg = process_radarcape(&msg, idx);
                    if tx.send(msg).await.is_err() {
                        break 'receive;
                    }
                }
            }
        }
        Err(err_tcp) => {
            match UdpSocket::bind(&address).await {
                Ok(socket) => {
                    let msg_stream =
                        beast::next_msg(DataSource::Udp(socket)).await;
                    pin_mut!(msg_stream); // needed for iteration
                    'receive: loop {
                        while let Some(msg) = msg_stream.next().await {
                            let msg = process_radarcape(&msg, idx);
                            if tx.send(msg).await.is_err() {
                                break 'receive;
                            }
                        }
                    }
                }
                Err(err_udp) => {
                    panic!(
                        "Failed to connect in TCP ({}) and UDP ({})",
                        err_tcp, err_udp
                    );
                }
            }
        }
    }
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before unix epoch")
        .as_micros()
}

fn today() -> u128 {
    86_400 * (now() / 86_400_000_000)
}

fn process_radarcape(msg: &[u8], idx: usize) -> TimedMessage {
    // Copy the bytes from the slice into the array starting from index 2
    let mut array = [0u8; 8];
    array[2..8].copy_from_slice(&msg[2..8]);

    let ts = u64::from_be_bytes(array);
    let seconds = ts >> 30;
    let nanos = ts & 0x00003FFFFFFF;
    let ts = seconds as f64 + nanos as f64 * 1e-9;
    let frame = msg[9..]
        .iter()
        .map(|&b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("");
    let now = now() as f64 * 1e-6;

    let timestamp = today() as f64 + ts;
    let timesource = match (now - timestamp).abs() {
        value if value < 5. => TimeSource::Radarcape,
        _ => TimeSource::System,
    };
    // In some cases, the timestamp is just the one of dump1090, so forget it!
    let timestamp = match timesource {
        TimeSource::Radarcape => timestamp,
        TimeSource::System => now,
        TimeSource::External => panic!(), // impossible here
    };

    TimedMessage {
        timestamp,
        timesource,
        frame,
        message: None,
        idx,
    }
}
