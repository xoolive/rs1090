use crate::prelude::*;
use futures_util::pin_mut;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::{net::TcpStream, sync::mpsc};

pub async fn receiver(address: String) -> mpsc::Receiver<TimedMessage> {
    let (tx, rx) = mpsc::channel(100);

    match TcpStream::connect(address).await {
        Ok(stream) => {
            tokio::spawn(async move {
                let msg_stream = beast::next_msg(stream).await;
                pin_mut!(msg_stream); // needed for iteration
                loop {
                    while let Some(msg) = msg_stream.next().await {
                        let msg = process_radarcape(&msg);
                        tx.send(msg).await.expect("Failed to send message");
                    }
                }
            });
        }
        Err(e) => {
            panic!("Failed to connect: {}", e);
        }
    }
    rx
}

fn today() -> i64 {
    86_400
        * (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("SystemTime before unix epoch")
            .as_secs() as i64
            / 86_400)
}

fn process_radarcape(msg: &[u8]) -> TimedMessage {
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

    TimedMessage {
        timestamp: today() as f64 + ts,
        frame,
        message: None,
    }
}
