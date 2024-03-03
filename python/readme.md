# rs1090

rs1090 is a Python binding to the [rs1090](https://docs.rs/rs1090/) Rust library to decode Mode S and ADS-B messages.

It takes its inspiration from the Python [pyModeS](https://github.com/junzis/pyModeS) library. The direction ambitioned by rs1090 boil down to:

- improving the performance of Mode S decoding in Python;
- exporting trajectory data to cross-platform formats such as JSON or parquet;
- providing efficient multi-receiver Mode S decoding;
- serving real-time enriched trajectory data to external applications.

## Installation

```sh
pip install rs1090
```

## Usage

```pycon
>>> import rs1090
>>> rs1090.decode("8c4841753a9a153237aef0f275be")
{'DF': 'ADSB', 'icao24': '484175', 'BDS': '0,6', 'NUCp': 7, 'groundspeed': 17.0, 'track': 92.8125, 'parity': 'odd', 'lat_cpr': 39195, 'lon_cpr': 110320}
```
