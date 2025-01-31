#![allow(rustdoc::missing_crate_level_docs)]

mod utils;

use js_sys::Object;
use rs1090::decode::bds::bds05::AirbornePosition;
use rs1090::decode::bds::bds10::DataLinkCapability;
use rs1090::decode::bds::bds17::CommonUsageGICBCapabilityReport;
use rs1090::decode::bds::bds18::GICBCapabilityReportPart1;
use rs1090::decode::bds::bds19::GICBCapabilityReportPart2;
use rs1090::decode::bds::bds20::AircraftIdentification;
use rs1090::decode::bds::bds21::AircraftAndAirlineRegistrationMarkings;
use rs1090::decode::bds::bds30::ACASResolutionAdvisory;
use rs1090::decode::bds::bds40::SelectedVerticalIntention;
use rs1090::decode::bds::bds44::MeteorologicalRoutineAirReport;
use rs1090::decode::bds::bds45::MeteorologicalHazardReport;
use rs1090::decode::bds::bds50::TrackAndTurnReport;
use rs1090::decode::bds::bds60::HeadingAndSpeedReport;
use rs1090::decode::cpr::{
    airborne_position_with_reference, surface_position_with_reference,
};
use rs1090::prelude::*;
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    set_panic_hook();
    Ok(())
}

struct DecodeError(DekuError);

impl From<DecodeError> for JsError {
    fn from(error: DecodeError) -> Self {
        JsError::new(&format!("{}", error.0))
    }
}

fn decode_message_with_reference(me: &mut ME, reference: [f64; 2]) {
    let [latitude_ref, longitude_ref] = reference;
    match me {
        ME::BDS05(airborne) => {
            if let Some(pos) = airborne_position_with_reference(
                airborne,
                latitude_ref,
                longitude_ref,
            ) {
                airborne.latitude = Some(pos.latitude);
                airborne.longitude = Some(pos.longitude);
            }
        }
        ME::BDS06(surface) => {
            if let Some(pos) = surface_position_with_reference(
                surface,
                latitude_ref,
                longitude_ref,
            ) {
                surface.latitude = Some(pos.latitude);
                surface.longitude = Some(pos.longitude);
            }
        }
        _ => (),
    }
}

#[wasm_bindgen]
pub fn decode(
    msg: &str,
    reference: Option<Vec<f64>>,
) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match Message::try_from(bytes.as_slice()) {
        Ok(mut msg) => {
            if let Some(reference) = reference.map(|v| [v[0], v[1]]) {
                match &mut msg.df {
                    ExtendedSquitterTisB { cf, .. } => {
                        decode_message_with_reference(&mut cf.me, reference)
                    }
                    ExtendedSquitterADSB(adsb) => {
                        decode_message_with_reference(
                            &mut adsb.message,
                            reference,
                        )
                    }
                    _ => {}
                }
            }
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(Object::from_entries(&map_result).unwrap().into())
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds05(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    let tc = &bytes[4] >> 3;
    if (9..22).contains(&tc) && tc != 19 {
        match AirbornePosition::from_bytes((&bytes[4..], 0)) {
            Ok((_, msg)) => {
                let map_result = serde_wasm_bindgen::to_value(&msg)?;
                Ok(map_result)
            }
            Err(e) => Err(DecodeError(e).into()),
        }
    } else {
        let msg = format!(
            "Invalid typecode {} for BDS 0,5 (9 to 18 or 20 to 22)",
            tc
        );
        Err(DecodeError(DekuError::Assertion(msg.into())).into())
    }
}

#[wasm_bindgen]
pub fn decode_bds10(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match DataLinkCapability::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds17(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match CommonUsageGICBCapabilityReport::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds18(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match GICBCapabilityReportPart1::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds19(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match GICBCapabilityReportPart2::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds20(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match AircraftIdentification::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds21(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match AircraftAndAirlineRegistrationMarkings::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds30(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match ACASResolutionAdvisory::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds40(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match SelectedVerticalIntention::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds44(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match MeteorologicalRoutineAirReport::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds45(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match MeteorologicalHazardReport::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds50(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match TrackAndTurnReport::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds60(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    match HeadingAndSpeedReport::from_bytes((&bytes[4..], 0)) {
        Ok((_, msg)) => {
            let map_result = serde_wasm_bindgen::to_value(&msg)?;
            Ok(map_result)
        }
        Err(e) => Err(DecodeError(e).into()),
    }
}

#[wasm_bindgen]
pub fn decode_bds65(msg: &str) -> Result<JsValue, JsError> {
    let bytes = hex::decode(msg)?;
    let tc = &bytes[4] >> 3;
    let enum_id = &bytes[4] & 0b111;
    match (tc, enum_id) {
        (31, id) if id < 2 => {
            match AircraftOperationStatus::from_bytes((&bytes[4..], 0)) {
                Ok((_, msg)) => {
                    let map_result = serde_wasm_bindgen::to_value(&msg)?;
                    Ok(map_result)
                }
                Err(e) => Err(DecodeError(e).into()),
            }
        }
        _ => {
            let msg = format!(
                "Invalid typecode {} (31) or category {} (0 or 1) for BDS 6,5",
                tc, enum_id
            );
            Err(DecodeError(DekuError::Assertion(msg.into())).into())
        }
    }
}
