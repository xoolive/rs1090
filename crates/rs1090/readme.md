# rs1090

rs1090 is a Rust library to decode Mode S and ADS-B messages.

It takes its inspiration from the Python [pyModeS](https://github.com/junzis/pyModeS) library, and uses [deku](https://github.com/sharksforarms/deku) in order to decode binary data in a clean declarative way.

The project started as a fork of a very similar project called [adsb-deku](https://crates.io/crates/adsb_deku), but modules have been refactored to match [pyModeS](https://github.com/junzis/pyModeS) design, implementations extensively reviewed, simplified, corrected, and completed.

The direction ambitioned by rs1090 boil down to:

- improving the performance of Mode S decoding in Python;
- exporting trajectory data to cross-platform formats such as JSON or parquet;
- providing efficient multi-receiver Mode S decoding;
- serving real-time enriched trajectory data to external applications.

If you just want to decode ADS-B messages from your Raspberry and visualize the data on a map, you may want to stick to one of the dump0190 implementations.

The rs1090 library comes with a companion application [decode1090](https://crates.io/crates/decode1090) and a Python binding [rs1090](https://pypi.org/project/rs1090).

## Installation

Run the following Cargo command in your project directory:

```sh
cargo add rs1090
```

Or add the following line to your `Cargo.toml`:

```toml
rs1090 = "0.2.0"  # check for the latest version
```

For the Python binding:

```sh
pip install rs1090
```

## Usage

In Rust:

```rust
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
```

In Python:

```pycon
>>> import rs1090
>>> rs1090.decode("8c4841753a9a153237aef0f275be")
{'df': '17', 'icao24': '484175', 'bds': '06', 'NUCp': 7, 'groundspeed': 17.0, 'track': 92.8125, 'parity': 'odd', 'lat_cpr': 39195, 'lon_cpr': 110320}
```

For large sets of messages in Python (e.g. what you can download through [pyopensky](https://github.com/open-aviation/pyopensky)):

```pycon
>>> import rs1090
>>> rs1090.decode(msg_list, ts_list)  # includes CPR to position decoding
...
>>> rs1090.decode(msg_list, ts_list, reference=(lat0, lon0))  # useful for surface messages
...
```
