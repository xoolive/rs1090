use deku::DekuContainerRead;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use rs1090::decode::Message;
use rs1090::source::beast::next_beast_msg;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;

fn today() -> i64 {
    86_400
        * (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("SystemTime before unix epoch")
            .as_secs() as i64
            / 86_400)
}

#[derive(Serialize)]
struct TimedMessage<'a> {
    timestamp: f64,

    //#[serde(skip)]
    frame: &'a String,

    #[serde(flatten)]
    message: Message,
}

fn process_radarcape(msg: &[u8]) {
    // Copy the bytes from the slice into the array starting from index 2
    let mut array = [0u8; 8];
    array[2..8].copy_from_slice(&msg[2..8]);

    let ts = u64::from_be_bytes(array);
    let seconds = ts >> 30;
    let nanos = ts & 0x00003FFFFFFF;
    let ts = seconds as f64 + nanos as f64 * 1e-9;
    let frame = Message::from_bytes((&msg[9..], 0)).unwrap().1;

    let msg = TimedMessage {
        timestamp: today() as f64 + ts,
        frame: &msg[9..]
            .iter()
            .map(|&b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join(""),
        message: frame,
    };

    /*println!(
        "{:?} {}\n{:#?}",
        today() as f64 + ts,
        msg.frame,
        msg.message
    );*/

    println!(
        "{}",
        serde_json::to_string(&msg).expect("Failed to serialize")
    );
}

#[tokio::main]
async fn main() {
    let server_address = "radarcape:10005";

    match TcpStream::connect(server_address).await {
        Ok(stream) => {
            let message_stream = next_beast_msg(stream).await;
            pin_mut!(message_stream); // needed for iteration
            while let Some(msg) = message_stream.next().await {
                process_radarcape(&msg);
            }
        }
        Err(e) => {
            panic!("Failed to connect: {}", e);
        }
    }
}
