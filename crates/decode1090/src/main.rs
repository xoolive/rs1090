#![doc = include_str!("../readme.md")]

use clap::Parser;
use deku::DekuContainerRead;
use futures_util::pin_mut;
use rs1090::decode::cpr::{decode_position, AircraftState, Position};
use rs1090::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
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

    /// Activate JSON output
    #[arg(long, default_value = "false")]
    json: bool,

    /// Reference coordinates for the decoding
    ///  (e.g. --latlon 43.3,1.35 or --latlon ' -34,18.6' if negative)
    #[arg(long, default_value=None)]
    latlon: Option<Position>,

    /// Individual messages to decode
    msgs: Vec<String>,
}

#[tokio::main]
async fn main() {
    let options = Options::parse();

    if !options.msgs.is_empty() {
        for msg in options.msgs {
            let bytes = hex::decode(&msg).unwrap();
            let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
            println!("{}", serde_json::to_string(&msg).unwrap());
        }
        return;
    }

    let server_address = format!("{}:{}", options.host, options.port);
    let mut reference = options.latlon;

    match TcpStream::connect(server_address).await {
        Ok(stream) => {
            let mut aircraft: BTreeMap<ICAO, AircraftState> = BTreeMap::new();
            let msg_stream = beast::next_msg(stream).await;
            pin_mut!(msg_stream); // needed for iteration
            while let Some(msg) = msg_stream.next().await {
                if let Some(mut msg) = process_radarcape(&msg) {
                    match &mut msg.message.df {
                        ExtendedSquitterADSB(adsb) => decode_position(
                            &mut adsb.message,
                            msg.timestamp,
                            &adsb.icao24,
                            &mut aircraft,
                            &mut reference,
                        ),
                        ExtendedSquitterTisB { cf, .. } => decode_position(
                            &mut cf.me,
                            msg.timestamp,
                            &cf.aa,
                            &mut aircraft,
                            &mut reference,
                        ),
                        _ => {}
                    };
                    if options.json {
                        println!("{}", serde_json::to_string(&msg).unwrap());
                    }
                }
            }
        }
        Err(e) => {
            panic!("Failed to connect: {}", e);
        }
    }
}
