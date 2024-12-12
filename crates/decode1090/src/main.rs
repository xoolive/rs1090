#![doc = include_str!("../readme.md")]

use clap::Parser;
use rs1090::decode::cpr::{decode_position, AircraftState, Position, UpdateIf};
use rs1090::decode::SensorMetadata;
use rs1090::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use tokio::fs::{self, File};
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
    ///  (e.g. --reference LFPG for major airports,
    ///   --reference 43.3,1.35 or --reference ' -34,18.6' if negative)
    #[arg(long, short, default_value=None)]
    reference: Option<Position>,

    /// Output file instead of stdout
    #[arg(long, short, default_value=None)]
    output: Option<String>,

    /// Deduplication threshold (in ms)
    #[arg(long, short, default_value = "400")]
    deduplication: u128,

    /// Individual messages to decode
    msgs: Vec<String>,
}

// We create this struct because it is too troublesome to have Deserialize for
// Message at this point.
#[derive(Serialize, Deserialize)]
struct JSONEntry {
    timestamp: f64,
    rssi: Option<f32>, // from older format
    #[serde(
        serialize_with = "rs1090::decode::as_hex",
        deserialize_with = "rs1090::decode::from_hex"
    )]
    frame: Vec<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    metadata: Vec<SensorMetadata>,
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

    let mut reference = options.reference;
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

        let mut cache: HashMap<Vec<u8>, Vec<JSONEntry>> = HashMap::new();
        // Need to do timestamps in u128 because f64 is not comparable (Ord)
        let mut expiration_heap: BinaryHeap<Reverse<(u128, Vec<u8>)>> =
            BinaryHeap::new();

        let update_reference = Some(Box::new(|pos: &AirbornePosition| {
            pos.alt.is_some_and(|alt| alt < 1000)
        })
            as Box<dyn Fn(&AirbornePosition) -> bool>);

        // Print the JSON objects
        for mut json in json_objects.into_iter().flatten() {
            // In case there is a rssi field (older version), create a source
            if json.rssi.is_some() {
                json.metadata.push(SensorMetadata {
                    system_timestamp: json.timestamp,
                    gnss_timestamp: None,
                    nanoseconds: None,
                    rssi: json.rssi,
                    serial: 0,
                    name: None,
                })
            }
            let timestamp_ms = (json.timestamp * 1e3) as u128;
            let frame = json.frame.clone();

            // Push the JSON to the list of similar messages received
            cache.entry(frame.clone()).or_default().push(json);

            // Push the expiration timestamp into the heap
            if cache[&frame].len() == 1 {
                expiration_heap.push(Reverse((
                    timestamp_ms + options.deduplication,
                    frame.clone(),
                )));
            }

            // Check and handle expired entries
            while let Some(Reverse((curtime, frame))) = expiration_heap.pop() {
                if curtime > timestamp_ms {
                    // If not expired, push it back and stop processing
                    expiration_heap.push(Reverse((curtime, frame)));
                    break;
                }

                // Otherwise clear the cache and process the deduplicated message
                if let Some(entries) = cache.remove(&frame) {
                    let _ = process_entries(
                        entries,
                        &mut aircraft,
                        &mut reference,
                        &update_reference,
                        &mut output_file,
                    )
                    .await;
                }
            }
        }
        // Flush remaining entries after processing all lines
        while let Some(Reverse((_curtime, frame))) = expiration_heap.pop() {
            if let Some(entries) = cache.remove(&frame) {
                let _ = process_entries(
                    entries,
                    &mut aircraft,
                    &mut reference,
                    &update_reference,
                    &mut output_file,
                )
                .await;
            }
        }
    }

    if !options.msgs.is_empty() {
        for msg in options.msgs {
            let bytes = hex::decode(&msg).unwrap();
            let msg = Message::try_from(bytes.as_slice()).unwrap();
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

// Helper function to merge entries into a single output
async fn process_entries(
    mut entries: Vec<JSONEntry>,
    aircraft: &mut BTreeMap<ICAO, AircraftState>,
    reference: &mut Option<Position>,
    update_reference: &UpdateIf,
    mut output_file: &mut Option<File>,
) -> Result<(), Box<dyn std::error::Error>> {
    let merged_metadata: Vec<SensorMetadata> = entries
        .iter()
        .flat_map(|entry| entry.metadata.clone())
        .collect();
    let json = entries.first_mut().unwrap();

    let message = if let Ok((_, msg)) = Message::from_bytes((&json.frame, 0)) {
        Some(msg)
    } else {
        None
    };

    // If old fashioned file, include the data in a metadata entry
    let mut msg = TimedMessage {
        timestamp: json.timestamp,
        frame: json.frame.clone(),
        message,
        metadata: merged_metadata,
        decode_time: None,
    };
    if let Some(message) = &mut msg.message {
        match &mut message.df {
            ExtendedSquitterADSB(adsb) => decode_position(
                &mut adsb.message,
                msg.timestamp,
                &adsb.icao24,
                aircraft,
                reference,
                update_reference,
            ),
            ExtendedSquitterTisB { cf, .. } => decode_position(
                &mut cf.me,
                msg.timestamp,
                &cf.aa,
                aircraft,
                reference,
                update_reference,
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
    Ok(())
}
