#![allow(rustdoc::missing_crate_level_docs)]

use pyo3::prelude::*;
use rayon::prelude::*;
use rs1090::decode::cpr::{decode_positions, Position};
use rs1090::prelude::*;

#[pyfunction]
fn decode_one(msg: String) -> PyResult<Vec<u8>> {
    let bytes = hex::decode(msg).unwrap();
    if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
        let pkl = serde_pickle::to_vec(&msg, Default::default()).unwrap();
        Ok(pkl)
    } else {
        Ok([128, 4, 78, 46].to_vec()) // None
    }
}

#[pyfunction]
fn decode_vec_time(
    msgs: Vec<String>,
    ts: Vec<f64>,
    reference: Option<[f64; 2]>,
) -> PyResult<Vec<u8>> {
    let mut res = Vec::<TimedMessage>::with_capacity(msgs.len());
    for (msg, timestamp) in msgs.iter().zip(ts) {
        let bytes = hex::decode(msg).unwrap();
        if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
            res.push(TimedMessage {
                timestamp,
                message: msg,
            });
        }
    }
    if let Some(position) = reference {
        decode_positions(
            &mut res,
            Some(Position {
                latitude: position[0],
                longitude: position[1],
            }),
        );
    }
    let pkl = serde_pickle::to_vec(&res, Default::default()).unwrap();
    Ok(pkl)
}

#[pyfunction]
fn decode_vec(msgs: Vec<String>) -> PyResult<Vec<u8>> {
    let mut res = Vec::<Message>::with_capacity(msgs.len());
    for msg in msgs {
        let bytes = hex::decode(msg).unwrap();
        if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
            res.push(msg);
        }
    }
    let pkl = serde_pickle::to_vec(&res, Default::default()).unwrap();
    Ok(pkl)
}

#[pyfunction]
fn decode_parallel(msgs_set: Vec<Vec<String>>) -> PyResult<Vec<u8>> {
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
fn decode_parallel_time(
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

/// A Python module implemented in Rust.
#[pymodule]
fn _rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_one, m)?)?;
    m.add_function(wrap_pyfunction!(decode_vec, m)?)?;
    m.add_function(wrap_pyfunction!(decode_vec_time, m)?)?;
    m.add_function(wrap_pyfunction!(decode_parallel, m)?)?;
    m.add_function(wrap_pyfunction!(decode_parallel_time, m)?)?;
    Ok(())
}
