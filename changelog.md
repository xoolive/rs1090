# Changelog

## Unreleased

- Fix BDS 0,6 ground speed formula: movement codes 13-38 now use correct 0.5 kt steps instead of 0.25 kt per ICAO Doc 9871
  - Affected taxiing aircraft and slow ground movements (2-15 kt range)
- Add comprehensive ICAO Doc 9871 specification documentation across all 20 BDS decoder files
  - Every BDS register now includes ICAO table references and section numbers
  - All field encodings documented with formulas, ranges, and LSB resolutions
- Fix altitude decoding to support negative altitudes for below-sea-level airports (EHAM, etc.) (#422, #424)
- Fix altitude field 0x000 (all-zeros) to return None instead of 0 per DO-260B spec (#425, #426)
  - Affects ~1.3% of Mode S messages with altitude unavailable
- **BREAKING**: Change AC13Field to wrap Option<i32> instead of i32 for proper "altitude unavailable" handling
- Fix invalid Gillham altitude patterns returning misleading 0 ft instead of None
- Reinforce pressure decode logic
- Implement no_history_expire option (fix #391)
- Include README in Python package distribution
- Fix Docker release script
- Fix permission issue on npm.js publishing
- Slow down flake.lock auto-updates
- Remove unnecessary key with trusted publisher
- Fix tests after breaking changes

## 0.4.15

- Repository renamed from `rs1090` to `jet1090`
- Add Python 3.14 support (#402, #404)
- Upgrade deku from 0.18.1 to 0.20.2 (#380)
- New logo for the project
- Update platform name for macOS Intel builds
- Fail with clear error message if no data source is provided (fix #365)
- Serialization of reserved BDS 6,5 messages (#337, #338)
- Remove NODE_AUTH_TOKEN from npm publish workflow
- Revert to fixed gain for RTL-SDR ADS-B reception
- Trigger workflow on push to master only
- Multiple dependency updates and flake.lock updates

## 0.4.14

- Fix flake.nix for Linux and macOS compatibility
- Fix compilation issues with Rust 1.90.0
- Set automatic gain for RTL-SDR
- Update syntax for dev dependencies
- Fix for macOS linking issues in Nix flake
- Move aircraft database to rs1090 core library
- Fix WASM dependency issues
- Bump tonic and prost versions
- Bump Rust version
- Include docs.rs link in README
- Update cargo-dist and maturin
- Code refactoring

## 0.4.13

- Switch to ubuntu-22.04 for GitHub Actions runner

## 0.4.12

- Switch to ubuntu-22.04 for builds

## 0.4.11

- Internal release infrastructure updates

## 0.4.10

- Internal release infrastructure updates

## 0.4.9

- Add ssh tunnelling for Websocket and SeRo sources (#199, #200)
- Add support for ProxyCommand in SSH configuration
- Reuse existing SSH connections for efficiency
- Upgrade makiko library, try several identities for SSH authentication
- Fix documentation for sources
- Fix link in documentation

## 0.4.8

- Add option --update-db to download the latest aircraft database
- Add ssh tunnelling for TCP sources (#193)
- Fix for ubuntu-22.04 due to libsoapy version constraints
- Preparation for release

## 0.4.7

- Adjust filter behaviour for redis output
- Add French stride mappings for aircraft registrations
- Remove libssl dependency on Ubuntu, include arm64 target (#188)
- Publish to ghcr.io including arm64 support
- Add ghcr.io container image for both amd64 and arm64 architectures
- Remove South African stride mapping (no longer in use)
- Default filter behaviour adjustments

## 0.4.6

- Build target fixes

## 0.4.5

- Corner-case numerical problem in CPR decoding fix #153 (#154)
- Decode position from CPR with reference in Python binding (#163)
- Test suite for WASM bindings (#164)
- Update reference altitude threshold to below 5000ft (previously 1000ft was too low for some airports)
- Filter on SeRo sensors
- Add /airport route to REST API
- Add /track?icao24=...&since=... API endpoint with timestamp filtering
- Add airports and aircraft information query support in WASM binding
- Query airports functionality in WASM
- Refactor aircraft_information/pattern function for WASM API
- Upgrade cargo-dist with fix for issue #872
- GitHub Actions workflow fixes

## 0.4.4

- Dependency adjustments

## 0.4.3

- Add search bar (regex accepted) in jet1090 TUI based on callsign, icao24, registration, typecode, and receptor name
- Filter icao24 addresses and DF (Downlink Format) types in output
- Sensors/Receptors now have altitude field
- Catch more errors for BDS 0,5 and 6,5 decoding
- Improve error messages on various BDS types
- Switch to stdio for TUI, configure logging for soapysdr (#147)
- Possibility to filter messages by DF and icao24 address
- Publish to ghcr.io (fix #127)
- Add documentation for Arch Linux installation (#145)
- Remove unsafe code from codebase
- Improve README documentation
- Trigger Docker/GHCR actions on release tags

## 0.4.2

- Fix BDS 6,5 bug for DF 21 messages
- Implement WebAssembly bindings (#139)
- Move documentation pages to new structure
- Initial commit for jet1090 user documentation site
- Include shell completion for nushell
- Update Python version support

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
