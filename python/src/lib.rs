#![allow(rustdoc::missing_crate_level_docs)]

use pyo3::prelude::*;
use rs1090::prelude::*;

#[pyfunction]
fn decode(msg: String) -> PyResult<Vec<u8>> {
    let bytes = hex::decode(msg).unwrap();
    if let Ok((_, msg)) = Message::from_bytes((&bytes, 0)) {
        let mbp = rmp_serde::to_vec_named(&msg).unwrap();
        Ok(mbp)
    } else {
        Ok([0xc0].to_vec())
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn _rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    Ok(())
}
