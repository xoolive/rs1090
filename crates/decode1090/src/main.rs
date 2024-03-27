#![doc = include_str!("../readme.md")]

use clap::Parser;
use rs1090::decode::cpr::{decode_position, AircraftState, Position};
use rs1090::prelude::*;
use std::collections::BTreeMap;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Parser)]
#[command(
    name = "decode1090",
    version,
    author = "xoolive",
    about = "Decode Mode S demodulated raw messages to JSON format"
)]
struct Options {
    /// Address of the demodulating server (beast feed)
    #[arg(long, default_value=None)]
    host: Option<String>,

    /// Port of the demodulating server
    #[arg(long, default_value=None)]
    port: Option<u16>,

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse();

    let mut file = if let Some(output_path) = options.output {
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

    if !options.msgs.is_empty() {
        for msg in options.msgs {
            let bytes = hex::decode(&msg).unwrap();
            let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
            let json = serde_json::to_string(&msg).unwrap();
            if let Some(file) = &mut file {
                file.write_all(json.as_bytes()).await?;
                file.write_all("\n".as_bytes()).await?;
            } else {
                println!("{}", json);
            }
        }
        return Ok(());
    }

    let mut reference = options.latlon;
    let mut aircraft: BTreeMap<ICAO, AircraftState> = BTreeMap::new();

    if let Some(host) = options.host {
        if let Some(port) = options.port {
            let server_address = format!("{}:{}", host, port);
            let mut rx = radarcape::receiver(server_address).await;

            while let Some(tmsg) = rx.recv().await {
                let frame = hex::decode(&tmsg.frame).unwrap();
                if let Ok((_, msg)) = Message::from_bytes((&frame, 0)) {
                    let mut msg = TimedMessage {
                        timestamp: tmsg.timestamp,
                        frame: tmsg.frame.to_string(),
                        message: Some(msg),
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
                            _ => {}
                        }
                    };
                    let json = serde_json::to_string(&msg).unwrap();
                    if let Some(file) = &mut file {
                        file.write_all(json.as_bytes()).await?;
                        file.write_all("\n".as_bytes()).await?;
                    } else {
                        println!("{}", json);
                    }
                }
            }
        }
    }
    Ok(())
}
