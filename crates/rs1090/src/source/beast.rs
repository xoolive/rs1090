use async_stream::stream;
use futures::stream::SplitStream;
use futures_util::stream::{Stream, StreamExt};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio_tungstenite::{
    tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error};

use std::collections::HashSet;

/// Iterate a Beast binary feed.
///
///  - esc "1" : 6 byte MLAT timestamp, 1 byte signal level, 2 byte Mode-AC
///  - esc "2" : 6 byte MLAT timestamp, 1 byte signal level, 7 byte Mode-S short frame
///  - esc "3" : 6 byte MLAT timestamp, 1 byte signal level, 14 byte Mode-S long frame
///  - esc "4" : 6 byte MLAT timestamp, status data, DIP switch configuration settings (not on Mode-S Beast classic)
///
/// esc esc: true 0x1a
/// esc is 0x1a, and "1", "2" and "3" are 0x31, 0x32 and 0x33
///
/// Decoding the timestamp:
/// <https://wiki.modesbeast.com/Radarcape:Firmware_Versions#The_GPS_timestamp>

pub type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub enum DataSource {
    Tcp(TcpStream),
    Udp(UdpSocket),
    Ws(WsStream),
}

pub async fn next_msg(mut stream: DataSource) -> impl Stream<Item = Vec<u8>> {
    // Initialize a HashSet to check for valid message types
    let valid_msg_types: HashSet<u8> =
        vec![0x31, 0x32, 0x33, 0x34].into_iter().collect();

    let mut data = Vec::new();
    stream! {
    loop {
        // Read from the stream into the buffer
        let mut buffer = [0u8; 1024];
        let bytes_read = match &mut stream {
            DataSource::Tcp(tcp_stream) => {
                match tcp_stream.read(&mut buffer).await {
                    Ok(0) => break, // Connection closed by peer
                    Ok(n) => n,
                    Err(e) => {
                        error!("Error reading from socket: {}", e);
                        break;
                    }
                }
            }
            DataSource::Udp(udp_socket) => {
                match udp_socket.recv_from(&mut buffer).await {
                    Ok((n, _)) => n,
                    Err(e) => {
                        error!("Error reading from socket: {}", e);
                        break;
                    }
                }
            }
            DataSource::Ws(ws_receive) => {
                match ws_receive.next().await {
                    Some(Ok(Message::Binary(data))) => {
                        debug!("Received {:?}", data);
                        let len = data.len().min(buffer.len());
                        buffer[..len].copy_from_slice(&data[..len]);
                        len
                    }
                    _ => {
                        error!("Error reading from websocket");
                        break;
                    }
                }
            }
        };

        // Extend the data vector with the read bytes
        data.extend_from_slice(&buffer[..bytes_read]);

        while data.len() >= 23 {
            if let Some(it) = data.iter().position(|&x| x == 0x1A) {
                data = data.split_off(it);

                if data.len() < 23 {
                    break;
                }

                let msg_type = data[1];
                if valid_msg_types.contains(&msg_type) {
                    // Collapse consecutive 0x1A into a single 0x1A
                    let mut ref_idx = 1;
                    let mut idx;
                    let msg_size = match msg_type {
                        0x31 => 11,
                        0x32 => 16,
                        0x33 => 23,
                        0x34 => 23, // Adjust the message size accordingly
                        _ => 0,
                    };

                    loop {
                        idx = data[ref_idx..msg_size.min(data.len())]
                            .iter()
                            .position(|&x| x == 0x1A);
                        if let Some(start) = idx.map(|idx| ref_idx + idx) {
                            ref_idx = start + 1;
                            if data.get(ref_idx) == Some(&0x1A) {
                                data.splice(start..=start, std::iter::empty());
                            }
                        } else {
                            break;
                        }
                    }

                    if idx.is_some() || data.len() < msg_size {
                        // Move to the next buffer
                        break;
                    }

                    let msg = data.drain(..msg_size).collect::<Vec<u8>>();
                    if msg_type != 0x34 {
                        yield msg
                    }
                } else {
                    // Probably corrupted message
                    data = data.split_off(1);
                }
            } else {
                break;
            }
        }
    }
    }
}
