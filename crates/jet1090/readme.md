# jet1090

jet1090 is an improved version of [dump1090](https://github.com/flightaware/dump1090/) in Rust designed to decode in parallel several sources of Mode S and ADS-B data (Beast feed in TCP/UDP, RTL-SDR) and serve the info in REST and ZMQ endpoints.

## Installation

Run the following Cargo command:

```sh
cargo install jet1090
```

## Support for RTL-SDR dongles

MacOS:

brew install soapysdr
brew install soapyrtlsdr

Linux:

apt install libsoapysdr-dev for building
apt install soapysdr-module-rtlsdr for running

Windows:

The pre-built Windows Pothos SDR development environment ships the necessary DLLs necessary to decode from the RTL-SDR. Follow the [instructions](https://github.com/pothosware/PothosSDR/wiki/Tutorial) for "Download and Install" and use Zadig to recognise your USB dongle.

## Usage

See `--help` for more information.
