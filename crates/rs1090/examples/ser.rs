use hexlit::hex;
use rs1090::prelude::*;
use tracing::error;
use serde::{Serialize};
use rmp_serde::{ Serializer};
use std::env;
use std::io::Cursor;

fn main() {
    // Read RUST_LOG environment variable and proceed accordingly
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let format = if args.len() > 1 {
        match args[1].as_str() {
            "--json" => "json",
            "--msgpack" => "msgpack",
            _ => {
                println!("Usage: {} [--json|--msgpack]", args[0]);
                println!("Default format: msgpack");
                "msgpack"
            }
        }
    } else {
        // Default to msgpack if no format is specified
        "msgpack"
    };

    let bytes: [u8; 14] = hex!("8d4bb463003d10000000001b5bec");

    // ADS-B decoding
    match Message::from_bytes((&bytes, 0)) {
        Ok((_, msg)) => {
            match format {
                "json" => {
                    // JSON output
                    let json = serde_json::to_string_pretty(&msg).unwrap();
                    println!("JSON Output:");
                    println!("{}", json);

                    // JSON deserialization
                    println!("\nJSON Deserialization:");
                    match serde_json::from_str::<Message>(&json) {
                        Ok(deserialized_msg) => {
                            println!("Successfully deserialized from JSON!");
                            println!("Deserialized message: {:?}", deserialized_msg);
                        },
                        Err(e) => println!("Failed to deserialize JSON: {}", e),
                    }
                },
                "msgpack" => {
                    // MessagePack output
                    let mut buf = Vec::new();
                    msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
                    println!("MessagePack Output:");
                    println!("Binary: {:?}", buf);
                    println!("Size: {} bytes", buf.len());

                    // MessagePack deserialization
                    println!("\nMessagePack Deserialization:");
                    match rmp_serde::from_read::<_, Message>(Cursor::new(&buf)) {
                        Ok(deserialized_msg) => {
                            println!("Successfully deserialized from MessagePack!");
                            println!("Deserialized message: {:?}", deserialized_msg);

                            // Verify original and deserialized are equal
                            if format!("{:?}", msg) == format!("{:?}", deserialized_msg) {
                                println!("Original and deserialized messages match!");
                            } else {
                                println!("Warning: Original and deserialized messages differ!");
                                println!("Original    : {:?}", msg);
                                println!("Deserialized: {:?}", deserialized_msg);
                            }
                        },
                        Err(e) => println!("Failed to deserialize MessagePack: {}", e),
                    }
                },
                _ => unreachable!(),
            }
        }
        Err(e) => error!("{}", e.to_string()),
    }

    // Print usage help
    println!("\nNote: Run with --json for JSON format or --msgpack for MessagePack format");
}
