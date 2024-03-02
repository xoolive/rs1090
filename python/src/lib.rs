use pyo3::prelude::*;
use rs1090::prelude::*;

#[pyfunction]
fn decode(msg: String) -> PyResult<Vec<u8>> {
    let bytes = hex::decode(&msg).unwrap();
    let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
    let mbp = rmp_serde::to_vec_named(&msg).unwrap();
    Ok(mbp)
    /*let mpb = match Message::from_bytes((&bytes, 0)) {
        Ok((_, msg)) => {
            // Serialize the struct to MessagePack
            rmp_serde::to_vec_named(&msg).unwrap()
        }
        Err(_error) => [192].to_vec(), // None
    };
    Ok(mpb)*/
}

/// A Python module implemented in Rust.
#[pymodule]
fn _rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    Ok(())
}
