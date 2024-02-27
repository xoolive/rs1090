#![doc = include_str!("../readme.md")]

extern crate alloc;

use alloc::fmt;
use clap::Parser;
use deku::DekuContainerRead;
use futures_util::pin_mut;
use rs1090::prelude::*;
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
struct TimedMessage {
    timestamp: f64,

    //#[serde(skip)]
    frame: String,

    #[serde(flatten)]
    message: Message,
}

impl fmt::Display for TimedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:.0} {}", &self.timestamp, &self.frame)?;
        writeln!(f, "{}", &self.message)
    }
}
impl fmt::Debug for TimedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:.0} {}", &self.timestamp, &self.frame)?;
        writeln!(f, "{:#}", &self.message)
    }
}

fn process_radarcape(msg: &[u8]) -> Option<TimedMessage> {
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
    if let Ok((_, msg)) = Message::from_bytes((&msg[9..], 0)) {
        Some(TimedMessage {
            timestamp: today() as f64 + ts,
            frame,
            message: msg,
        })
    } else {
        None
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "decode1090",
    version,
    author = "xoolive",
    about = "Decode Mode S demodulated raw messages"
)]
struct Options {
    /// Address of the demodulating server (beast feed)
    #[arg(long, default_value = "radarcape")]
    host: String,

    /// Port of the demodulating server
    #[arg(short, long, default_value = "10005")]
    port: u16,

    /// Activate debug output of messages (deactivate JSON)
    #[arg(long, default_value = "false")]
    debug: bool,

    /// Individual messages to decode
    msgs: Vec<String>,
}

#[tokio::main]
async fn main() {
    let options = Options::parse();

    if !options.msgs.is_empty() {
        for msg in options.msgs {
            let bytes = hex::decode(&msg).unwrap();
            let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
            if options.debug {
                println!("{}", msg);
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&msg).expect("Failed to serialize")
                );
            }
        }
        return;
    }

    let server_address = format!("{}:{}", options.host, options.port);

    match TcpStream::connect(server_address).await {
        Ok(stream) => {
            let msg_stream = beast::next_msg(stream).await;
            pin_mut!(msg_stream); // needed for iteration
            while let Some(msg) = msg_stream.next().await {
                if let Some(msg) = process_radarcape(&msg) {
                    if options.debug {
                        println!("{}", msg);
                    } else {
                        println!(
                            "{}",
                            serde_json::to_string(&msg)
                                .expect("Failed to serialize")
                        );
                    }
                }
            }
        }
        Err(e) => {
            panic!("Failed to connect: {}", e);
        }
    }
}
