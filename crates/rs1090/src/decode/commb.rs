use super::bds::bds10::DataLinkCapability;
use super::bds::bds17::GICBCapabilityReport;
use super::bds::bds20::AircraftIdentification;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Comm-B Data Selector (BDS)
 *
 * The first four BDS codes (1,0, 1,7, 2,0, 3,0) belong to the ELS service,
 * the next three ones (4,0, 5,0, 6,0) belong to the EHS services,
 * and the last two codes (4,4, 4,5) report meteorological information.
 */

#[derive(Debug, PartialEq, DekuRead, Serialize, Clone)]
pub struct DataSelector {
    #[deku(reader = "read_bds10(deku::input_bits)")]
    #[serde(rename = "1,0", skip_serializing_if = "Option::is_none")]
    pub bds10: Option<DataLinkCapability>,

    #[deku(reader = "read_bds17(deku::input_bits)")]
    #[serde(rename = "1,7", skip_serializing_if = "Option::is_none")]
    pub bds17: Option<GICBCapabilityReport>,

    #[deku(reader = "read_bds20(deku::input_bits)")]
    #[serde(rename = "2,0", skip_serializing_if = "Option::is_none")]
    pub bds20: Option<AircraftIdentification>,
}

fn read_bds10(
    input: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<DataLinkCapability>), DekuError> {
    let (_, bytes, _) = input.domain().region().unwrap();

    if let Ok((_, bds10)) = DataLinkCapability::from_bytes((bytes, 0)) {
        Ok((input, Some(bds10)))
    } else {
        Ok((input, None))
    }
}

fn read_bds17(
    input: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<GICBCapabilityReport>), DekuError> {
    let (_, bytes, _) = input.domain().region().unwrap();

    if let Ok((_, bds17)) = GICBCapabilityReport::from_bytes((bytes, 0)) {
        Ok((input, Some(bds17)))
    } else {
        Ok((input, None))
    }
}

fn read_bds20(
    input: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<AircraftIdentification>), DekuError> {
    let (_, bytes, _) = input.domain().region().unwrap();

    if let Ok((_, bds20)) = AircraftIdentification::from_bytes((bytes, 0)) {
        Ok((input, Some(bds20)))
    } else {
        Ok((input, None))
    }
}
