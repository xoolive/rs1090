#![doc = include_str!("../readme.md")]

use clap::Parser;
use flate2::read::GzDecoder;
use rs1090::decode::commb::MessageProcessor;
use rs1090::decode::cpr::{decode_position, AircraftState, Position, UpdateIf};
use rs1090::decode::SensorMetadata;
use rs1090::prelude::*;
use serde::{Deserialize, Serialize};
use sevenz_rust::SevenZReader;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::io::Read;
use std::path::Path;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

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

/// Read and decompress input file based on extension
async fn read_input_file(
    input_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(input_path);
    let extension = path.extension().and_then(|e| e.to_str());

    match extension {
        Some("7z") => {
            // Read .7z file directly into memory without temp files
            let mut archive =
                SevenZReader::open(path, sevenz_rust::Password::empty())?;
            let mut content = String::new();

            // Read first entry in archive (assume single .jsonl file inside)
            archive.for_each_entries(|_entry, reader| {
                reader.read_to_string(&mut content)?;
                Ok(false) // Stop after first entry
            })?;

            Ok(content)
        }
        Some("gz") => {
            // Decompress .gz file using flate2
            let file = std::fs::File::open(path)?;
            let mut decoder = GzDecoder::new(file);
            let mut content = String::new();
            decoder.read_to_string(&mut content)?;
            Ok(content)
        }
        _ => {
            // Regular file - read directly
            Ok(tokio::fs::read_to_string(path).await?)
        }
    }
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

/// Parse Beast format from CSV line (timestamp,hexdata)
/// Beast format: 0x1a \[type\] \[6-byte timestamp\] \[1-byte signal\] \[message\]
///  - 0x1a 0x32: Mode-S short (16 bytes total, 7-byte message)
///  - 0x1a 0x33: Mode-S long (23 bytes total, 14-byte message)
fn parse_beast_csv_line(line: &str) -> Option<JSONEntry> {
    let parts: Vec<&str> = line.trim().split(',').collect();
    if parts.len() != 2 {
        return None;
    }

    let timestamp: f64 = parts[0].parse().ok()?;
    let hex_data = hex::decode(parts[1]).ok()?;

    // Check Beast format: starts with 0x1a
    if hex_data.is_empty() || hex_data[0] != 0x1a {
        return None;
    }

    // Check message type and extract Mode S message
    let frame = match hex_data.get(1) {
        Some(0x32) if hex_data.len() >= 16 => {
            // Mode-S short: 0x1a 0x32 [6-byte ts] [1-byte signal] [7-byte message]
            hex_data[9..16].to_vec()
        }
        Some(0x33) if hex_data.len() >= 23 => {
            // Mode-S long: 0x1a 0x33 [6-byte ts] [1-byte signal] [14-byte message]
            hex_data[9..23].to_vec()
        }
        _ => return None,
    };

    Some(JSONEntry {
        timestamp,
        rssi: None,
        frame,
        metadata: vec![],
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse();

    let input_file = options.input;

    let mut output_file = if let Some(output_path) = options.output {
        Some(
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(output_path)
                .await?,
        )
    } else {
        None
    };

    let mut reference = options.reference;
    let mut aircraft: BTreeMap<ICAO, AircraftState> = BTreeMap::new();

    if let Some(input_path) = input_file {
        let content_str = read_input_file(&input_path).await?;

        let raw_messages: Vec<&str> = content_str.split('\n').collect();

        // Detect format: CSV (Beast) or JSONL
        // CSV format: timestamp,hexdata (e.g., "1539371430.786029,1a330000519c8b74068c3c648d...")
        // JSONL format: {"timestamp":...,"frame":"..."}
        let is_csv = raw_messages
            .iter()
            .find(|line| !line.trim().is_empty())
            .map(|first_line| {
                !first_line.trim().starts_with('{')
                    && first_line.contains(',')
                    && first_line.split(',').count() == 2
            })
            .unwrap_or(false);

        // Parse each line based on detected format
        let mut json_objects: Vec<JSONEntry> = if is_csv {
            // Parse CSV (Beast format)
            raw_messages
                .iter()
                .filter(|line| !line.trim().is_empty())
                .filter_map(|line| parse_beast_csv_line(line))
                .collect()
        } else {
            // Parse JSONL format
            raw_messages
                .iter()
                .filter_map(|msg| serde_json::from_str(msg).ok())
                .collect()
        };

        // Sort messages by timestamp to ensure chronological processing
        // This is critical for CPR decoding which uses reference positions
        // from previous messages. Out-of-order messages cause wrong reference
        // positions, especially for surface messages (TC 5-8, BDS 06).
        json_objects.sort_by(|a, b| {
            a.timestamp
                .partial_cmp(&b.timestamp)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut cache: HashMap<Vec<u8>, Vec<JSONEntry>> = HashMap::new();
        // Need to do timestamps in u128 because f64 is not comparable (Ord)
        let mut expiration_heap: BinaryHeap<Reverse<(u128, Vec<u8>)>> =
            BinaryHeap::new();

        let update_reference = Some(Box::new(|pos: &AirbornePosition| {
            pos.alt.is_some_and(|alt| alt < 1000)
        })
            as Box<dyn Fn(&AirbornePosition) -> bool>);

        // Print the JSON objects
        for mut json in json_objects.into_iter() {
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
                println!("{json}");
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
        // Decode positions for ADS-B messages
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
            _ => {}
        }

        // Sanitize Comm-B messages
        MessageProcessor::new(message, aircraft)
            .sanitize_commb()
            .finish();

        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Serialization error: {}", e);
                eprintln!("Message timestamp: {}", msg.timestamp);
                eprintln!("Frame: {}", hex::encode(&msg.frame));
                eprintln!("Message: {:?}", msg.message);
                panic!("Failed to serialize message");
            }
        };
        if let Some(file) = &mut output_file {
            file.write_all(json.as_bytes()).await?;
            file.write_all("\n".as_bytes()).await?;
        } else {
            println!("{json}");
        }
    }
    Ok(())
}
