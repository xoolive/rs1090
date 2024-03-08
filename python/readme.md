# rs1090

rs1090 is a Python binding to the [rs1090](https://docs.rs/rs1090/) Rust library to decode Mode S and ADS-B messages.

It takes its inspiration from the Python [pyModeS](https://github.com/junzis/pyModeS) library. The direction ambitioned by rs1090 boils down to:

- improving the performance of Mode S decoding in Python;
- exporting trajectory data to cross-platform formats such as JSON or parquet;
- providing efficient multi-receiver Mode S decoding;
- serving real-time enriched trajectory data to external applications.

## Installation

```sh
pip install rs1090
```

## Usage

For single messages:

```pycon
>>> import rs1090
>>> rs1090.decode("8c4841753a9a153237aef0f275be")
{'df': '17', 'icao24': '484175', 'bds': '06', 'NUCp': 7, 'groundspeed': 17.0, 'track': 92.8125, 'parity': 'odd', 'lat_cpr': 39195, 'lon_cpr': 110320}
```

For batches of messages:

```pycon
>>> import rs1090
>>> rs1090.decode(msg_list)
...
>>> rs1090.decode(msg_list, ts_list)  # includes CPR to position decoding
...
>>> rs1090.decode(msg_list, ts_list, reference=(lat0, lon0))  # useful for surface messages
...
```
