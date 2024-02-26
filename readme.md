# rs1090

rs1090 is a Rust library to decode Mode S and ADS-B messages.

It takes its inspiration from the Python [pyModeS](https://github.com/junzis/pyModeS) library, and uses [deku](https://github.com/sharksforarms/deku) in order to decode binary data in a clean declarative way.

The project started as a fork of a very similar project called [adsb-deku](https://github.com/rsadsb/adsb_deku), but modules have been refactored to match [pyModeS](https://github.com/junzis/pyModeS) design, implementations extensively reviewed, simplified, corrected, and completed.

The direction ambitioned by rs1090 boil down to:

- improving the performance of Mode S decoding in Python;
- exporting trajectory data to cross-platform formats such as JSON or parquet;
- providing efficient multi-receiver Mode S decoding;
- serving real-time enriched trajectory data to external applications.

If you just want to decode ADS-B messages from your Raspberry and visualize the data on a map, you may want to stick to one of the dump0190 implementations.

## Usage

- As a Rust library, decoding goes as suggested by the deku library:

  ```rs
  use deku::prelude::*;
  use hexlit::hex;
  use rs1090::decode::Message;

  // rs1090 manipulates Vec<u8>
  let bytes = hex!("8c4841753a9a153237aef0f275be");
  // ADS-B decoding
  let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
  // JSON output
  let json = serde_json::to_string(&msg).expect("Failed to serialize");
  ```

- As an executable, regardless the options, it can be convenient to pipe the output to the [jq](https://github.com/jqlang/jq) command-line JSON processor.

  ```sh
  # decode the Beast output from your radarcape
  > cargo run --bin decode1090 -- --host radarcape --port 10005
  {"timestamp":1708901277.8567717,"frame":"8d4d224260595215b81666e59d7a","DF":"ADSB","icao24":"4d2242","BDS":"0,5","NUCp":6,"NICb":0,"altitude":16725,"source":"barometric","odd_flag":"even","lat_cpr":68316,"lon_cpr":5734}
  {"timestamp":1708901277.858925,"frame":"2000179f86b805","DF":"DF4","altitude":36975,"icao24":"86b805"}
  {"timestamp":1708901277.8650618,"frame":"8f400f02990c5c32f80c94b9ad6f","DF":"ADSB","icao24":"400f02","BDS":"0,9","NACv":1,"groundspeed":416.07,"track":347.37,"vrate_src":"GNSS","vertical_rate":-128,"geo_minus_baro":-475}
  (...)
  ```

  ```sh
  # decode individual message
  > cargo run --bin decode1090 -- 5d3c66e6c6ad01 8d3c66e699086a919838884331c7 8d3c66e6580fb120964f56c5c8ef 8d3c66e6ea07e7d0013c083f1ab0 | jq .
  {
    "DF": "DF11",
    "capability": "airborne",
    "icao24": "3c66e6"
  }
  {
    "DF": "ADSB",
    "icao24": "3c66e6",
    "BDS": "0,9",
    "NACv": 1,
    "groundspeed": 174.2,
    "track": 142.93,
    "vrate_src": "GNSS",
    "vertical_rate": -832,
    "geo_minus_baro": -175
  }
  {
    "DF": "ADSB",
    "icao24": "3c66e6",
    "BDS": "0,5",
    "NUCp": 7,
    "NICb": 0,
    "altitude": 2075,
    "source": "barometric",
    "odd_flag": "even",
    "lat_cpr": 36939,
    "lon_cpr": 20310
  }
  {
    "DF": "ADSB",
    "icao24": "3c66e6",
    "BDS": "6,2",
    "source": "MCP/FCU",
    "selected_altitude": 4000,
    "barometric_setting": 999.2,
    "NACp": 9,
    "tcas_operational": true
  }
  ```
