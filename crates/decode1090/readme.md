# decode1090

decode1090 is the companion application to the [rs1090](https://crates.io/crates/rs1090) Rust library, designed to decode Mode S and ADS-B messages.

## Installation

Run the following Cargo command:

```sh
cargo install decode1090
```

## Usage

See `--help` for more information.

Output filenames ending in `.jsonl.zst` automatically enable streaming Zstandard
compression (including `file`, individual-message and CAT48 output). Other names
remain uncompressed. Output is flushed approximately once per second, including
while idle, and finalized on normal completion.

```sh
decode1090 file flight.jsonl.zst --reference LFBO --update-position --output replay.jsonl.zst
decode1090 follow replay.jsonl.zst
```

`follow` copies existing records from the first byte, then waits for new bytes;
it does not re-decode frames. It supports plain JSONL and concatenated Zstandard
frames, including a still-open frame. Stop with Ctrl-C. The file must already
exist; rotation/replacement and truncation are not supported.

Unlike `tail -c +1 -f file.jsonl.zst | zstd -dc`, which can buffer piped input
and lag at low traffic rates, `follow` reads incremental chunks and immediately
flushes decoded output. Downstream programs can still add their own buffering.
Forced process termination can leave an unfinished frame; normal completion
writes its footer.

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

- Decode files. The `file` subcommand accepts JSONL and Beast CSV inputs directly, including compressed `.gz`, `.zst` and `.7z` files such as `.jsonl.zst`, `.jsonl.7z` and `.csv.7z`.

  ```sh
  decode1090 file flight.jsonl.zst --reference LFBO --output decoded.jsonl
  decode1090 file flight.jsonl.7z --reference LFBO --output decoded.jsonl
  decode1090 file raw_beast.csv.7z --reference LFPO --output decoded.jsonl
  ```

  For a moving receiver recorded by `jet1090 --update-position`, replay using
  the same per-sensor reference policy (airborne updates below 5000 ft):

  ```sh
  decode1090 file flight.jsonl --reference LFBO --update-position --output replay.jsonl
  ```

  By default, file replay preserves recorded arrival order (`--input-order=true`),
  including small timestamp inversions, and disables deduplication
  (`--deduplication 0`). To sort by timestamp, use `--input-order=false`.
  Nonzero deduplication requires sorting; for example,
  `--input-order=false --deduplication 400` restores the previous defaults.
  Without `--update-position`, per-message reference updates are
  disabled; the periodic lowest-aircraft reference refresh remains active, as
  in jet1090. Exact historical re-decoding also depends on the original decoder
  version and state: intentional validation fixes may change old hypotheses.

  File replay always re-decodes the raw frames; there is no enrichment-only
  shortcut. Output paths that alias an input (including hard links) are rejected.
