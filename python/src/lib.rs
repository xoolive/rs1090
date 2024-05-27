#![allow(rustdoc::missing_crate_level_docs)]

use std::collections::HashMap;

use pyo3::prelude::*;
use rayon::prelude::*;
use regex::Regex;
use rs1090::data::patterns::PATTERNS;
use rs1090::data::tail::tail;
use rs1090::decode::cpr::{decode_positions, Position};
use rs1090::decode::flarm::Flarm;
use rs1090::decode::TimeSource;
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
                    if let Ok((_, message)) = Message::from_bytes((&bytes, 0)) {
                        Some(TimedMessage {
                            timestamp,
                            timesource: TimeSource::External,
                            frame: msg.to_string(),
                            message: Some(message),
                            idx: 0,
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

#[pyfunction]
fn aircraft_information(
    icao24: &str,
    registration: Option<&str>,
) -> PyResult<HashMap<String, String>> {
    let mut reg = HashMap::<String, String>::new();
    let hexid = u32::from_str_radix(icao24, 16)?;

    reg.insert("icao24".to_string(), icao24.to_lowercase());

    if let Some(tail) = tail(hexid) {
        reg.insert("registration".to_string(), tail);
    }
    if let Some(tail) = registration {
        reg.insert("registration".to_string(), tail.to_string());
    }

    if let Some(pattern) = &PATTERNS.registers.iter().find(|elt| {
        if let Some(start) = &elt.start {
            if let Some(end) = &elt.end {
                let start = u32::from_str_radix(&start[2..], 16).unwrap();
                let end = u32::from_str_radix(&end[2..], 16).unwrap();
                return (hexid >= start) & (hexid <= end);
            }
        }
        false
    }) {
        reg.insert("country".to_string(), pattern.country.to_string());
        reg.insert("flag".to_string(), pattern.flag.to_string());
        if let Some(p) = &pattern.pattern {
            reg.insert("pattern".to_string(), p.to_string());
        }
        if let Some(comment) = &pattern.comment {
            reg.insert("comment".to_string(), comment.to_string());
        }

        if let Some(tail) = reg.get("registration") {
            if let Some(categories) = &pattern.categories {
                if let Some(cat) = categories.iter().find(|elt| {
                    let re = Regex::new(&elt.pattern).unwrap();
                    re.is_match(tail)
                }) {
                    reg.insert("pattern".to_string(), cat.pattern.to_string());
                    if let Some(category) = &cat.category {
                        reg.insert(
                            "category".to_string(),
                            category.to_string(),
                        );
                    }
                    if let Some(country) = &cat.country {
                        reg.insert("country".to_string(), country.to_string());
                    }
                    if let Some(flag) = &cat.flag {
                        reg.insert("flag".to_string(), flag.to_string());
                    }
                }
            }
        }
    } else {
        reg.insert("country".to_string(), "Unknown".to_string());
        reg.insert("flag".to_string(), "🏳".to_string());
    }

    Ok(reg)
}

/// A Python module implemented in Rust.
#[pymodule]
fn _rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Decoding functions
    m.add_function(wrap_pyfunction!(decode_1090, m)?)?;
    m.add_function(wrap_pyfunction!(decode_1090_vec, m)?)?;
    m.add_function(wrap_pyfunction!(decode_1090t_vec, m)?)?;
    m.add_function(wrap_pyfunction!(decode_flarm, m)?)?;
    m.add_function(wrap_pyfunction!(decode_flarm_vec, m)?)?;

    // icao24 functions
    m.add_function(wrap_pyfunction!(aircraft_information, m)?)?;

    Ok(())
}
