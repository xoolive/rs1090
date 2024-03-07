use hexlit::hex;
use rs1090::prelude::*;

fn main() {
    let bytes: [u8; 14] = hex!("8d4bb463003d10000000001b5bec");
    // ADS-B decoding
    if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
        // JSON output
        let json = serde_json::to_string(&msg).unwrap();
        println!("{}", json);
    }
}
