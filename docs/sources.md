# Sources of data

`jet1090` decodes feeds of Mode S messages coming from a variety of sources.

## RTL-SDR dongles

The most common source is the RTL-SDR dongle.

![rtl-sdr dongle](images/rtlsdr.png){: style="height:150px"}

`jet1090` must be [compiled with the `rtlsdr` feature](install.md) in order to support RTL-SDR dongles.

You can check whether `jet1090` detects your dongle:

```sh
$ jet1090 --discover=rtlsdr
Detached kernel driver
Found Rafael Micro R820T tuner
Reattached kernel driver
[INFO] Opening Generic RTL2832U OEM :: 00000001...
Detached kernel driver
Found Rafael Micro R820T tuner
Reattached kernel driver
```

You can then start decoding incoming messages. Use the `--verbose` option to check it is working well.

```sh
$ jet1090 --verbose rtlsdr://
Found Rafael Micro R820T tuner
[INFO] Opening Generic RTL2832U OEM :: 00000001...
Found Rafael Micro R820T tuner
[INFO] Using format CS16.
Allocating 15 zero-copy buffers

{"timestamp":1735082050.8826132,"frame":"8d34768d58b524b5986843dfee26","df":"17","icao24":"34768d","bds":"05","tc":11,"NUCp":7,"NICb":0,"altitude":35050,"source":"barometric","parity":"odd","lat_cpr":23244,"lon_cpr":26691,"metadata":[{"system_timestamp":1735082050.8826132,"rssi":-28.450745,"serial":14924845721654670821,"name":"rtlsdr"}]}
{"timestamp":1735082051.040175,"frame":"8d34768de11200000000002919db","df":"17","icao24":"34768d","bds":"61","subtype":"emergency_priority","emergency_state":"none","squawk":"2010","metadata":[{"system_timestamp":1735082051.040175,"rssi":-29.93521,"serial":14924845721654670821,"name":"rtlsdr"}]}
```

!!! tip

    It is recommended to specify a reference location in order to decode ground messages as well.

    You may use four-letter airport codes, three-letter airport codes, or latitude and longitude values separated by a comma.

    ```sh
    jet1090 --verbose rtlsdr://@EHAM  # Amsterdam Schiphol airport (NL)
    jet1090 --verbose rtlsdr://@LHR  # London Heathrow airport (UK)
    ```

!!! tip

    By default, all dongles seem to have the same serial number `00000001`.

    If you want to give a different serial number to one of them, you may use the `rtl_eeprom` program:

    ```sh
    rtl_eeprom -d 0 -s 00000002
    ```

    If you have several dongles attached to different antennas, you may specify which one to select based on their serial number. That information will appear in the `"name"` field in the metadata:

    ```sh
    jet1090 rtlsdr://serial=00000001 rtlsdr://serial=00000002
    ```

## Beast format

### TCP

Most decoders share the data they decode in the Beast binary format which contains timestamp information, signal level information (rssi) and Mode S short or long messages.

- dump1090 sends the data from the 30005 port;
- Radarcape and Air!Squitter receivers (by [jetvision](https://jetvision.de/)) send their CRC pre-checked raw data from the 10003 port.

You can decode that data with the following command:

```sh
jet1090 --verbose tcp://192.168.0.20:10003
```

You can add more receivers to the command:

```sh
jet1090 --verbose tcp://192.168.0.20:10003@LFBO tcp://192.168.0.30:10003@LFPO
```

### UDP

When receivers are not accessible by the client for network configuration reasons, it is common practice to redirect the Beast feed to a UDP endpoint on another IP address. There is a dedicated setting on Radarcape devices for that. Otherwise, you can use `socat` to redirect the TCP output to an external UDP endpoint:

```sh
socat TCP:localhost:30005 UDP-DATAGRAM:1.2.3.4:5678
```

You can run `jet1090` on that remote machine:

```sh
jet1090 --verbose udp://:5678
```

!!! warning

    In UDP, only one program can listen to a specific port at a time. If you need multiple instances of a program to receive the same data feed, you can use the following [WebSocket](#websocket) solution with tools like `wsbroad` and `websocat`.

### WebSocket

The following settings helps to centralize several UDP feeds on one machine, and redistribute them.

First run the `wsbroad` command:

```sh
wsbroad 0.0.0.0:9876
```

Then redirect the UDP feed to a WebSocket endpoint:

```sh
websocat -b -u udp-l:127.0.0.1:5678 ws://0.0.0.0:9876/5678
```

You may then check data is coming:

```sh
websocat ws://localhost:9876/5678
```

!!! tip

    You may create some services for managing these connexions more easily. Copy the service files to `.config/systemd/user` then:

    ```sh
    systemctl --user start wsbroad
    systemctl --user start websocat@5678
    ```

    You can have a `websocat@.service` file as such:

    ```service
    [Unit]
    Description=Redirect UDP %i streaming to WebSocket

    [Service]
    Type=simple
    ExecStart=/home/user/.local/bin/websocat -b -u udp-l:127.0.0.1:%i ws://0.0.0.0:9876/%i
    RestartSec=3

    [Install]
    WantedBy=multi-user.target
    ```

    and a `wsbroad.service` as:

    ```service
    [Unit]
    Description=WebSocket broadcast at 9876

    [Service]
    Type=simple
    ExecStart=/home/user/.local/bin/wsbroad 0.0.0.0:9876
    Restart=always
    RestartSec=3

    [Install]
    WantedBy=multi-user.target
    ```

## SeRo Systems API

If you have a token for the [SeRo Systems API](https://doc.sero-systems.de/api/), include it in your [configuration file](config.md#sero-systems) with the `sero.token` entry.
