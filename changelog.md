# Changelog

## 0.4.3

- Add a search bar (regex accepted, based on callsign, icao24, registration, typecode and receptor name)
- Filter icao24 and DF types in output
- Sensors/Receptors have an altitude field
- Catch more error for BDS 0,5 and 6,5
- Improve error messages on some BDS types

## 0.4.2

- Fix BDS 6,5 bug for DF 21
- Implement WebAssembly bindings

## 0.4.1

- Fix building scripts in GitHub releases

## 0.4.0

- Basic deduplication algorithm
- Support for SeRo system data source
- Adjust name aliases and serial numbers
- Major refactoring

## 0.3.9

- New data model
- allow to specify which rtlsdr to use
- fix log_file being ignored in config files
- prefer XDG_CONFIG_HOME if defined over system default, useful in MacOS
- Improve BDS inference based on ground truth messages
- add option to prevent laptop from sleeping
- improve error message when redis server is not reachable

## 0.3.8

- Adapt to Python 3.13
- Propagate errors on non-null CRC chcks

## 0.3.7

- Switch to uv

## 0.3.6

- Select rtl-sdr with arguments
- Typing issue on Redis publish after upgrade

## 0.3.5

- Do not attempt to decode Beast 0x34 messages (avoid errors)
- Fix error forwarding

## 0.3.4

- Major code adjustments following upgrade to deku 0.17.0
- Upgrade cargo-dist, maturin, change version semantics (shared version)

## 0.3.0

- Adjust size for MPSC channels
- Decode Beast format from a websocket source
- Decode RSSI in Beast format, and implement it for RTL-SDR
- Improvement in CPR decoding
- Shell completion
- Support for nix/flake
