use crate::decode::time::{now_in_ns, since_today_to_nanos};
use crate::prelude::*;
use crate::source::beast::DataSource;
use futures_util::pin_mut;
use std::io;
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
            let tmsg = process_radarcape(&msg, serial, name.clone());
            info!("Received {}", tmsg);
            if tx.send(tmsg).await.is_err() {
                break 'receive;
            }
        }
    }
    Ok(())
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
    let seconds = ts_u64 as u128 >> 30;
    let nanos = ts_u64 & 0x00003FFFFFFF;
    let timestamp_in_s =
        since_today_to_nanos(seconds * 1_000_000_000 + nanos as u128) as f64
            * 1e-9;

    let system_timestamp = now_in_ns() as f64 * 1e-9;

    let gnss_timestamp = match (system_timestamp - timestamp_in_s).abs() {
        value if value < 3600. => Some(timestamp_in_s),
        _ => None,
    };

    let rssi = if msg[8] == 0xff { None } else { Some(msg[8]) };
    let rssi = rssi.map(|v| v as f64 / 255.);
    let rssi = rssi.map(|v| 10. * (v * v).log10() as f32);

    // In some cases, the timestamp is just the one of dump1090, so forget it!
    let metadata = SensorMetadata {
        system_timestamp,
        gnss_timestamp,
        nanoseconds: Some(ts_u64),
        rssi,
        serial,
        name,
    };

    TimedMessage {
        timestamp: metadata.system_timestamp,
        frame: msg[9..].to_vec(),
        message: None,
        metadata: vec![metadata],
        decode_time: None,
    }
}
