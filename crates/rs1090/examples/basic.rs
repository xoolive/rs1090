use hexlit::hex;
use rs1090::prelude::*;
use tracing::error;

fn main() {
    // Read RUST_LOG environment variable and proceed accordingly
    tracing_subscriber::fmt::init();

    let bytes: [u8; 14] = hex!("8d4bb463003d10000000001b5bec");

    // ADS-B decoding
    match Message::from_bytes((&bytes, 0)) {
        Ok((_, msg)) => {
            // JSON output
            let json = serde_json::to_string(&msg).unwrap();
            println!("{}", json);
        }
        Err(e) => error!("{}", e.to_string()),
    }

    // Equivalent way of decoding
    if let Ok(msg) = Message::try_from(bytes.as_slice()) {
        // JSON output
        let json = serde_json::to_string(&msg).unwrap();
        println!("{}", json);
    }
}
