#![allow(rustdoc::missing_crate_level_docs)]

use pyo3::prelude::*;
use rayon::prelude::*;
use rs1090::decode::cpr::{decode_positions, Position};
use rs1090::decode::flarm::Flarm;
use rs1090::prelude::*;

#[pyfunction]
fn decode_1090(msg: String) -> PyResult<Vec<u8>> {
    let bytes = hex::decode(msg).unwrap();
    if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
        let pkl = serde_pickle::to_vec(&msg, Default::default()).unwrap();
        Ok(pkl)
    } else {
        Ok([128, 4, 78, 46].to_vec()) // None
    }
}

#[pyfunction]
fn decode_1090_vec(msgs_set: Vec<Vec<String>>) -> PyResult<Vec<u8>> {
    let res: Vec<Option<Message>> = msgs_set
        .par_iter()
        .map(|msgs| {
            msgs.iter()
                .map(|msg| {
                    let bytes = hex::decode(msg).unwrap();
                    if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
                        Some(msg)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .flat_map(|v: Vec<Option<Message>>| v)
        .collect();
    let pkl = serde_pickle::to_vec(&res, Default::default()).unwrap();
    Ok(pkl)
}

#[pyfunction]
fn decode_1090t_vec(
    msgs_set: Vec<Vec<String>>,
    ts_set: Vec<Vec<f64>>,
    reference: Option<[f64; 2]>,
) -> PyResult<Vec<u8>> {
    let mut res: Vec<TimedMessage> = msgs_set
        .par_iter()
        .zip(ts_set)
        .map(|(msgs, ts)| {
            msgs.iter()
                .zip(ts)
                .filter_map(|(msg, timestamp)| {
                    let bytes = hex::decode(msg).unwrap();
                    if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
                        Some(TimedMessage {
                            timestamp,
                            message: msg,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        })
        .flat_map(|v: Vec<TimedMessage>| v)
        .collect();

    if let Some([latitude, longitude]) = reference {
        let position = Some(Position {
            latitude,
            longitude,
        });
        decode_positions(&mut res, position);
    }

    let pkl = serde_pickle::to_vec(&res, Default::default()).unwrap();
    Ok(pkl)
}

#[pyfunction]
fn decode_flarm(
    msg: String,
    ts: u32,
    reflat: f64,
    reflon: f64,
) -> PyResult<Vec<u8>> {
    let bytes = hex::decode(msg).unwrap();
    let reference = [reflat, reflon];
    if let Ok(msg) = Flarm::from_record(ts, &reference, &bytes) {
        let pkl = serde_pickle::to_vec(&msg, Default::default()).unwrap();
        Ok(pkl)
    } else {
        Ok([128, 4, 78, 46].to_vec()) // None
    }
}

#[pyfunction]
fn decode_flarm_vec(
    msgs_set: Vec<Vec<String>>,
    ts_set: Vec<Vec<u32>>,
    ref_lat: Vec<Vec<f64>>,
    ref_lon: Vec<Vec<f64>>,
) -> PyResult<Vec<u8>> {
    let reference: Vec<Vec<[f64; 2]>> = ref_lat
        .iter()
        .zip(ref_lon.iter())
        .map(|(lat, lon)| {
            lat.iter()
                .zip(lon.iter())
                .map(|(lat, lon)| [*lat, *lon])
                .collect()
        })
        .collect();
    let res: Vec<Flarm> = msgs_set
        .par_iter()
        .zip(ts_set)
        .zip(reference)
        .map(|((msgs, ts), reference)| {
            msgs.iter()
                .zip(ts)
                .zip(reference)
                .filter_map(|((msg, timestamp), reference)| {
                    let bytes = hex::decode(msg).unwrap();
                    if let Ok(flarm) =
                        Flarm::from_record(timestamp, &reference, &bytes)
                    {
                        Some(flarm)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .flat_map(|v: Vec<Flarm>| v)
        .collect();

    let pkl = serde_pickle::to_vec(&res, Default::default()).unwrap();
    Ok(pkl)
}
/// A Python module implemented in Rust.
#[pymodule]
fn _rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_1090, m)?)?;
    m.add_function(wrap_pyfunction!(decode_1090_vec, m)?)?;
    m.add_function(wrap_pyfunction!(decode_1090t_vec, m)?)?;
    m.add_function(wrap_pyfunction!(decode_flarm, m)?)?;
    m.add_function(wrap_pyfunction!(decode_flarm_vec, m)?)?;
    Ok(())
}
