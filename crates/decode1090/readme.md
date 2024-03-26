# decode1090

decode1090 is the companion application to the [rs1090](https://crates.io/crates/rs1090) Rust library, designed to decode Mode S and ADS-B messages.

MacOS:

brew install soapysdr
brew install soapyrtlsdr

Linux:

apt install libsoapysdr-dev for building
apt install soapysdr-module-rtlsdr for running

Windows:

The pre-built Windows Pothos SDR development environment ships the necessary DLLs necessary to decode from the RTL-SDR. Follow the [instructions](https://github.com/pothosware/PothosSDR/wiki/Tutorial) for "Download and Install" and use Zadig to recognise your USB dongle.

## Installation

Run the following Cargo command:

```sh
cargo install decode1090
```

## Usage

See `--help` for more information.

- Decode a Beast feed, coming from your radarcape for instance

  ```sh
  > decode1090 --host radarcape --port 10005
  {"timestamp":1708901277.8567717,"frame":"8d4d224260595215b81666e59d7a","df":"17","icao24":"4d2242","bds":"05","NUCp":6,"NICb":0,"altitude":16725,"source":"barometric","odd_flag":"even","lat_cpr":68316,"lon_cpr":5734}
  {"timestamp":1708901277.858925,"frame":"2000179f86b805","df":"4","altitude":36975,"icao24":"86b805"}
  {"timestamp":1708901277.8650618,"frame":"8f400f02990c5c32f80c94b9ad6f","df":"17","icao24":"400f02","bds":"09","NACv":1,"groundspeed":416.07,"track":347.37,"vrate_src":"GNSS","vertical_rate":-128,"geo_minus_baro":-475}
  (...)
  ```

- Decode individual messages.  
  Note how it can be convenient to pipe the output to the [jq](https://github.com/jqlang/jq) command-line JSON processor.

  ```sh
  > decode1090 5d3c66e6c6ad01 8d3c66e699086a919838884331c7 | jq .
  {
    "df": "11",
    "capability": "airborne",
    "icao24": "3c66e6"
  }
  {
    "df": "ADSB",
    "icao24": "3c66e6",
    "bds": "09",
    "NACv": 1,
    "groundspeed": 174.2,
    "track": 142.93,
    "vrate_src": "GNSS",
    "vertical_rate": -832,
    "geo_minus_baro": -175
  }
  ```
