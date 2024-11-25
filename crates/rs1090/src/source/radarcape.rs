use crate::prelude::*;
use crate::source::beast::DataSource;
use futures_util::pin_mut;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tracing::info;

pub enum BeastSource {
    TCP(String),
    UDP(String),
    Websocket(String),
}

pub async fn receiver(
    address: BeastSource,
    tx: mpsc::Sender<TimedMessage>,
    serial: u64,
    name: Option<String>,
) -> io::Result<()> {
    let msg_stream = match address {
        BeastSource::TCP(address) => match TcpStream::connect(&address).await {
            Ok(stream) => {
                info!("Connected to TCP stream: {}", address);
                DataSource::Tcp(stream)
            }
            Err(error) => {
                info!(
                    "Failed to connect to TCP {} ({}), trying in UDP",
                    address,
                    error.to_string()
                );
                DataSource::Udp(UdpSocket::bind(&address).await?)
            }
        },
        BeastSource::UDP(address) => {
            DataSource::Udp(UdpSocket::bind(&address).await?)
        }
        BeastSource::Websocket(address) => {
            info!("Connecting to websocket: {}", address);
            let (stream, _) = connect_async(&address)
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            info!("Connected to websocket: {}", address);
            let (_, rx) = stream.split();
            DataSource::Ws(rx)
        }
    };

    let msg_stream = beast::next_msg(msg_stream).await;
    pin_mut!(msg_stream); // needed for iteration
    'receive: loop {
        while let Some(msg) = msg_stream.next().await {
            let start = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("SystemTime before unix epoch")
                .as_secs_f64();
            let mut tmsg = process_radarcape(&msg, serial, name.clone());
            info!("Received {}", tmsg);
            if let Ok((_, msg)) = Message::from_bytes((&tmsg.frame, 0)) {
                tmsg.decode_time = Some(
                    SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("SystemTime before unix epoch")
                        .as_secs_f64()
                        - start,
                );
                tmsg.message = Some(msg);
                if tx.send(tmsg).await.is_err() {
                    break 'receive;
                }
            }
        }
    }
    Ok(())
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before unix epoch")
        .as_micros()
}

fn today(now: u128) -> u128 {
    86_400 * (now / 86_400)
}

fn process_radarcape(
    msg: &[u8],
    serial: u64,
    name: Option<String>,
) -> TimedMessage {
    // Copy the bytes from the slice into the array starting from index 2
    let mut array = [0u8; 8];
    array[2..8].copy_from_slice(&msg[2..8]);

    let ts_u64 = u64::from_be_bytes(array);
    let seconds = ts_u64 >> 30;
    let nanos = ts_u64 & 0x00003FFFFFFF;
    let ts = seconds as f64 + nanos as f64 * 1e-9;

    let rssi = if msg[8] == 0xff { None } else { Some(msg[8]) };
    let rssi = rssi.map(|v| v as f64 / 255.);
    let rssi = rssi.map(|v| 10. * (v * v).log10());

    /*let frame = msg[9..]
    .iter()
    .map(|&b| format!("{:02x}", b))
    .collect::<Vec<String>>()
    .join("");*/

    let now_u128 = now();
    let now = now_u128 as f64 * 1e-6;
    let timestamp = today(now_u128 / 1_000_000) as f64 + ts;

    /*let timesource = match (now - timestamp).abs() {
        value if value < 5. => TimeSource::Radarcape,
        _ => TimeSource::System,
    };*/
    // In some cases, the timestamp is just the one of dump1090, so forget it!
    /* let timestamp = match timesource {
        TimeSource::Radarcape => timestamp,
        TimeSource::System => now,
        TimeSource::External => panic!(), // impossible here
    };*/
    let metadata = SensorMetadata {
        system_timestamp: now,
        gnss_timestamp: match (now - timestamp).abs() {
            value if value < 3600. => Some(timestamp),
            _ => None,
        },
        rssi,
        serial,
        name,
    };

    TimedMessage {
        timestamp: metadata.gnss_timestamp.unwrap_or(metadata.system_timestamp),
        frame: msg[9..].to_vec(),
        message: None,
        metadata: vec![metadata],
        decode_time: None,
    }
}
