# decode1090

decode1090 is the companion application to the [rs1090](https://crates.io/crates/rs1090) Rust library, designed to decode Mode S and ADS-B messages.

## Installation

```sh
cargo install decode1090
```

## Usage

See `--help` for more information.

- Decode a Beast feed, coming from your radarcape for instance

  ```sh
  > cargo run --bin decode1090 -- --host radarcape --port 10005
  {"timestamp":1708901277.8567717,"frame":"8d4d224260595215b81666e59d7a","DF":"ADSB","icao24":"4d2242","BDS":"0,5","NUCp":6,"NICb":0,"altitude":16725,"source":"barometric","odd_flag":"even","lat_cpr":68316,"lon_cpr":5734}
  {"timestamp":1708901277.858925,"frame":"2000179f86b805","DF":"DF4","altitude":36975,"icao24":"86b805"}
  {"timestamp":1708901277.8650618,"frame":"8f400f02990c5c32f80c94b9ad6f","DF":"ADSB","icao24":"400f02","BDS":"0,9","NACv":1,"groundspeed":416.07,"track":347.37,"vrate_src":"GNSS","vertical_rate":-128,"geo_minus_baro":-475}
  (...)
  ```

- Decode individual messages.  
  Note how it can be convenient to pipe the output to the [jq](https://github.com/jqlang/jq) command-line JSON processor.

  ```sh
  > cargo run --bin decode1090 -- 5d3c66e6c6ad01 8d3c66e699086a919838884331c7 | jq .
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
  ```
