#![doc = include_str!("../readme.md")]

use clap::Parser;
use rs1090::decode::cpr::{decode_position, AircraftState, Position};
use rs1090::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Parser)]
#[command(
    name = "decode1090",
    version,
    author = "xoolive",
    about = "Decode Mode S demodulated raw messages to JSON format"
)]
struct Options {
    /// Input file instead of individual messages (jsonl format)
    #[arg(long, short, default_value= None)]
    input: Option<String>,

    /// Reference coordinates for the decoding
    ///  (e.g. --latlon LFPG for major airports,
    ///   --latlon 43.3,1.35 or --latlon ' -34,18.6' if negative)
    #[arg(long, default_value=None)]
    latlon: Option<Position>,

    /// Output file instead of stdout
    #[arg(long, short, default_value=None)]
    output: Option<String>,

    /// Individual messages to decode
    msgs: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct JSONEntry {
    timestamp: f64,
    timesource: rs1090::decode::TimeSource,
    rssi: Option<u8>,
    frame: String,
    idx: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse();

    let input_file = if let Some(input_path) = options.input {
        let file = fs::File::open(input_path).await?;
        Some(file)
    } else {
        None
    };

    let mut output_file = if let Some(output_path) = options.output {
        Some(
            fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(output_path)
                .await?,
        )
    } else {
        None
    };

    let mut reference = options.latlon;
    let mut aircraft: BTreeMap<ICAO, AircraftState> = BTreeMap::new();

    if let Some(mut file) = input_file {
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        let content_str = String::from_utf8_lossy(&contents);

        let raw_messages: Vec<&str> = content_str.split('\n').collect();

        // Parse each segment as a JSON object
        let json_objects: Vec<Result<JSONEntry, _>> = raw_messages
            .iter()
            .map(|msg| serde_json::from_str(msg))
            .collect();

        // Print the JSON objects
        for json in json_objects.into_iter().flatten() {
            let bytes = hex::decode(&json.frame).unwrap();
            let message = if let Ok((_, msg)) = Message::from_bytes((&bytes, 0))
            {
                Some(msg)
            } else {
                None
            };
            let mut msg = TimedMessage {
                timestamp: json.timestamp,
                timesource: json.timesource,
                rssi: json.rssi,
                frame: json.frame.to_string(),
                message,
                idx: json.idx.map_or(0, |idx| idx),
            };
            if let Some(message) = &mut msg.message {
                match &mut message.df {
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
                    CommBAltitudeReply { bds, .. } => {
                        if let (Some(_), Some(_)) = (&bds.bds50, &bds.bds60) {
                            bds.bds50 = None;
                            bds.bds60 = None
                        }
                    }
                    CommBIdentityReply { bds, .. } => {
                        if let (Some(_), Some(_)) = (&bds.bds50, &bds.bds60) {
                            bds.bds50 = None;
                            bds.bds60 = None
                        }
                    }
                    _ => {}
                }
                let json = serde_json::to_string(&msg).unwrap();
                if let Some(file) = &mut output_file {
                    file.write_all(json.as_bytes()).await?;
                    file.write_all("\n".as_bytes()).await?;
                } else {
                    println!("{}", json);
                }
            }
        }
    }
    // if let Ok((_, msg)) = Message::from_bytes((&frame, 0)) {
    //     let mut msg = TimedMessage {
    //         timestamp: tmsg.timestamp,
    //         timesource: tmsg.timesource,
    //         frame: tmsg.frame.to_string(),
    //         message: Some(msg),
    //         idx: tmsg.idx,
    //     };

    //     if let Some(message) = &mut msg.message {
    //         match &mut message.df {
    //             ExtendedSquitterADSB(adsb) => decode_position(
    //                 &mut adsb.message,
    //                 msg.timestamp,
    //                 &adsb.icao24,
    //                 &mut aircraft,
    //                 &mut reference,
    //             ),
    //             ExtendedSquitterTisB { cf, .. } => decode_position(
    //                 &mut cf.me,
    //                 msg.timestamp,
    //                 &cf.aa,
    //                 &mut aircraft,
    //                 &mut reference,
    //             ),
    //             CommBAltitudeReply { bds, .. } => {
    //                 if let (Some(_), Some(_)) = (&bds.bds50, &bds.bds60) {
    //                     bds.bds50 = None;
    //                     bds.bds60 = None
    //                 }
    //             }
    //             CommBIdentityReply { bds, .. } => {
    //                 if let (Some(_), Some(_)) = (&bds.bds50, &bds.bds60) {
    //                     bds.bds50 = None;
    //                     bds.bds60 = None
    //                 }
    //             }
    //             _ => {}
    //         }
    //         let json = serde_json::to_string(&msg).unwrap();
    //         if let Some(file) = &mut output_file {
    //             file.write_all(json.as_bytes()).await?;
    //             file.write_all("\n".as_bytes()).await?;
    //         } else {
    //             println!("{}", json);
    //         }
    //     }
    // }

    if !options.msgs.is_empty() {
        for msg in options.msgs {
            let bytes = hex::decode(&msg).unwrap();
            let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
            let json = serde_json::to_string(&msg).unwrap();
            if let Some(file) = &mut output_file {
                file.write_all(json.as_bytes()).await?;
                file.write_all("\n".as_bytes()).await?;
            } else {
                println!("{}", json);
            }
        }
    }

    Ok(())
}
