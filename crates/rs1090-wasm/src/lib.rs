mod utils;

use js_sys::Object;
use rs1090::prelude::*;
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

struct DecodeError(DekuError);

impl From<DecodeError> for JsError {
    fn from(error: DecodeError) -> Self {
        JsError::new(&format!("{}", error.0))
    }
}

#[wasm_bindgen]
pub fn decode(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match Message::try_from(bytes.as_slice()) {
        Ok(msg) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(Object::from_entries(&map_result).unwrap().into())
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    set_panic_hook();
    Ok(())
}
