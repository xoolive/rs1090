use num_complex::Complex;
use soapysdr::{configure_logging, Args, Device, Direction};
use tokio::sync::mpsc;

use crate::decode::time::now_in_ns;
use crate::prelude::*;
use crate::source::demod;
use std::fmt::{self, Display, Formatter};
use tracing::{error, info};

const DIRECTION: Direction = Direction::Rx;

pub async fn receiver<A: Into<Args> + fmt::Display + std::marker::Copy>(
    tx: mpsc::Sender<TimedMessage>,
    args: Option<A>,
    serial: u64,
    name: Option<String>,
) {
    match args {
        Some(args) => {
            info!("Trying to connect rtlsdr with options: {}", args)
        }
        None => info!("Trying to connect rtlsdr with options: driver=rtlsdr"),
    }
    configure_logging();
    let device = match args {
        None => Device::new("driver=rtlsdr"),
        Some(args) => Device::new(args),
    };

    let name = name.or(args
        .map(|a| Some(format!("{a}")))
        .unwrap_or(Some("rtlsdr".to_string())));

    let device = match device {
        Ok(device) => {
            info!("{:#}", device.hardware_info().unwrap());
            device
        }
        Err(error) => {
            eprintln!("SoapySDR error: {error}");
            std::process::exit(127);
        }
    };
    let channel = 0;
    device
        .set_frequency(DIRECTION, channel, demod::MODES_FREQ, ())
        .unwrap();
    device
        .set_sample_rate(DIRECTION, channel, demod::RTLSDR_RATE)
        .unwrap();
    device
        .set_gain_element(DIRECTION, channel, "TUNER", demod::RTLSDR_GAIN)
        .unwrap();

    let mut stream = device.rx_stream::<Complex<i16>>(&[channel]).unwrap();

    let mut buf = vec![Complex::new(0, 0); stream.mtu().unwrap()];
    stream.activate(None).unwrap();

    // Spawn a thread to send messages
    'receive: loop {
        match stream.read(&mut [&mut buf], 5_000_000) {
            Ok(len) => {
                let buf = &buf[..len];
                let outbuf = demod::magnitude(buf);
                let resulting_data = demod::demodulate2400(&outbuf).unwrap();
                for data in resulting_data {
                    let system_timestamp = now_in_ns() as f64 * 1e-9;
                    let metadata = SensorMetadata {
                        system_timestamp,
                        gnss_timestamp: None,
                        nanoseconds: None,
                        rssi: Some(10. * data.signal_level.log10() as f32),
                        serial,
                        name: name.clone(),
                    };
                    let tmsg = TimedMessage {
                        timestamp: system_timestamp,
                        frame: data.msg.to_vec(),
                        message: None,
                        metadata: vec![metadata],
                        decode_time: None,
                    };
                    if tx.send(tmsg).await.is_err() {
                        break 'receive;
                    }
                }
            }
            Err(e) => {
                error!("SoapySDR read error: {}", e);
            }
        }
    }
}

struct DisplayRange(Vec<soapysdr::Range>);

fn print_channel_info(
    dev: &Device,
    dir: Direction,
    channel: usize,
) -> Result<(), soapysdr::Error> {
    let dir_s = match dir {
        Direction::Rx => "Rx",
        Direction::Tx => "Tx",
    };
    info!("{} Channel {}", dir_s, channel);

    let freq_range = dev.frequency_range(dir, channel)?;
    info!("Freq range: {}", DisplayRange(freq_range));

    let sample_rates = dev.get_sample_rate_range(dir, channel)?;
    info!("Sample rates: {}", DisplayRange(sample_rates));

    info!("Antennas: ");
    for antenna in dev.antennas(dir, channel)? {
        info!("{}", antenna);
    }

    Ok(())
}

impl Display for DisplayRange {
    fn fmt(&self, w: &mut Formatter) -> fmt::Result {
        for (i, range) in self.0.iter().enumerate() {
            if i != 0 {
                write!(w, ", ")?
            }
            if range.minimum == range.maximum {
                write!(w, "{} MHz", range.maximum / 1e6)?
            } else {
                write!(
                    w,
                    "{} to {} MHz",
                    range.minimum / 1e6,
                    range.maximum / 1e6
                )?
            }
        }
        Ok(())
    }
}

pub fn enumerate(args: &str) {
    for devargs in
        soapysdr::enumerate(args).expect("SoapySDR: Error listing devices")
    {
        info!("Device: {}", devargs);

        if let Ok(dev) = soapysdr::Device::new(devargs) {
            info!("Hardware info: {:#}", dev.hardware_info().unwrap());

            for channel in 0..(dev.num_channels(Direction::Rx).unwrap_or(0)) {
                print_channel_info(&dev, Direction::Rx, channel)
                    .expect("Failed to get channel info");
            }

            for channel in 0..(dev.num_channels(Direction::Tx).unwrap_or(0)) {
                print_channel_info(&dev, Direction::Tx, channel)
                    .expect("Failed to get channel info");
            }
        }
    }
}
