use hexlit::hex;
use rs1090::prelude::*;

fn main() {
    let bytes: [u8; 14] = hex!("8c4841753a9a153237aef0f275be");
    // ADS-B decoding
    if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
        // JSON output
        let json = serde_json::to_string(&msg).expect("JSON error");
        println!("{}", json);
    }
}
