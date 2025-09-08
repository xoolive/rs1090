use core::alloc::Layout;
use std::alloc::alloc_zeroed;
use std::thread;

use rtl_sdr_rs::RtlSdr;
use rtl_sdr_rs::DEFAULT_BUF_LENGTH;
use tokio::sync::mpsc;
use tracing::error;
use tracing::info;

use crate::decode::time::now_in_ns;
use crate::decode::SensorMetadata;
use crate::decode::TimedMessage;
use crate::source::demod;

pub async fn receiver(
    tx: mpsc::Sender<TimedMessage>,
    serial: u64,
    name: Option<String>,
) {
    // Create a channel for communication between the non-async thread and this async function
    let (internal_tx, mut internal_rx) = mpsc::channel(32);

    // Spawn a standard thread to handle the RTL-SDR device which isn't Send-compatible
    let _handle = thread::spawn(move || {
        let mut sdr = match RtlSdr::open(0) {
            Ok(sdr) => sdr,
            Err(e) => {
                error!("Failed to open RTL-SDR device: {:?}", e);
                return;
            }
        };

        if let Err(e) = sdr.set_center_freq(demod::MODES_FREQ as u32) {
            error!("Failed to set frequency: {:?}", e);
            return;
        }

        if let Err(e) = sdr.set_sample_rate(demod::RTLSDR_RATE as u32) {
            error!("Failed to set sample rate: {:?}", e);
            return;
        }

        if let Err(e) = sdr.set_tuner_gain(rtl_sdr_rs::TunerGain::Auto) {
            error!("Failed to set tuner gain: {:?}", e);
            return;
        }

        if let Err(e) = sdr.set_bias_tee(false) {
            error!("Failed to disable bias-tee: {:?}", e);
            return;
        }

        if let Err(e) = sdr.reset_buffer() {
            error!("Failed to reset buffer: {:?}", e);
            return;
        }
        let mut buf: Box<[u8; DEFAULT_BUF_LENGTH]> = alloc_buf();

        'receive: loop {
            let n = match sdr.read_sync(&mut *buf) {
                Ok(n) => n,
                Err(e) => {
                    info!("Failed to read samples: {:?}", e);
                    break 'receive;
                }
            };

            if n < DEFAULT_BUF_LENGTH {
                info!("Short read ({:#?}), samples lost, exiting!", n);
                break 'receive;
            }

            // Convert raw bytes to complex samples
            let complex_samples: Vec<num_complex::Complex<i16>> = buf
                .chunks_exact(2)
                .map(|chunk| {
                    let real = chunk[0] as i16 - 127;
                    let imag = chunk[1] as i16 - 127;
                    num_complex::Complex::new(real, imag)
                })
                .collect();

            let outbuf = demod::magnitude(&complex_samples);
            let resulting_data = match demod::demodulate2400(&outbuf) {
                Ok(data) => data,
                Err(_) => continue,
            };

            // Send the demodulated data through the channel
            if internal_tx.blocking_send(resulting_data).is_err() {
                break 'receive;
            }
        }
    });

    // Process data from the internal channel in the async context
    while let Some(resulting_data) = internal_rx.recv().await {
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
                return;
            }
        }
    }
}

/// Allocate a buffer on the heap
fn alloc_buf<T>() -> Box<T> {
    let layout: Layout = Layout::new::<T>();
    // TODO move to using safe code once we can allocate an array directly on the heap.
    unsafe {
        let ptr = alloc_zeroed(layout) as *mut T;
        Box::from_raw(ptr)
    }
}
