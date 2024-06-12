# jet1090

jet1090 is an improved version of [dump1090](https://github.com/flightaware/dump1090/) in Rust designed to decode in parallel several sources of Mode S and ADS-B data (Beast feed in TCP/UDP, RTL-SDR) and serve the info in REST endpoints or websockets.

## Installation

Run the following Cargo command:

```sh
cargo install jet1090
```

You may also install already compiled versions from the [GitHub Releases](https://github.com/xoolive/rs1090/releases) page.

## Support for RTL-SDR dongles

Compile with the feature `rtlsdr`:

```sh
cargo install jet1090 --feature rtlsdr
```

- Recommended procedure with MacOS:

  ```sh
  brew install soapysdr
  brew install soapyrtlsdr
  ```

  Use the following commands if you want to keep your Mac awake while recording:

  ```sh
  caffeinate
  sudo pmset -b disablesleep 1
  ```

- Recommended procedure with Linux:

  ```sh
  apt install libsoapysdr-dev  # useful for building
  apt install soapysdr-module-rtlsdr  # useful for running
  ```

- Recommended procedure with Windows:

  The pre-built Windows Pothos SDR development environment ships the necessary DLLs necessary to decode from the RTL-SDR. Follow the [instructions](https://github.com/pothosware/PothosSDR/wiki/Tutorial) for "Download and Install" and use Zadig to recognise your USB dongle.

## Usage

See `--help` for more information.
