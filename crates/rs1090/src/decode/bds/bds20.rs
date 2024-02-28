use super::bds08;
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Aircraft identification (BDS 2,0)
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct AircraftIdentification {
    #[deku(bits = "8", map = "fail_if_not20")]
    #[serde(skip)]
    pub bds: u8,

    #[deku(reader = "bds08::callsign_read(deku::rest)")]
    pub callsign: String,
}

fn fail_if_not20(value: u8) -> Result<u8, DekuError> {
    if value == 0x20 {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "First bits must be 0x20 in BDS 2,0".to_string(),
        ))
    }
}
