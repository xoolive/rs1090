use super::bds::bds10::DataLinkCapability;
use super::bds::bds17::GICBCapabilityReport;
use super::bds::bds20::AircraftIdentification;
use super::bds::bds30::ACASResolutionAdvisory;
use super::bds::bds40::SelectedVerticalIntention;
use super::bds::bds44::MeteorologicalRoutineAirReport;
use super::bds::bds50::TrackAndTurnReport;
use super::bds::bds60::HeadingAndSpeedReport;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;
use std::fmt;

/**
 * ## Comm-B Data Selector (BDS)
 *
 * The first four BDS codes (1,0, 1,7, 2,0, 3,0) belong to the ELS service,
 * the next three ones (4,0, 5,0, 6,0) belong to the EHS services,
 * and the last two codes (4,4, 4,5) report meteorological information.
 */

#[derive(Debug, PartialEq, DekuRead, Serialize, Clone)]
pub struct DataSelector {
    #[deku(reader = "check_empty_bds(deku::input_bits)")]
    #[serde(skip)]
    /// Set to true if all zeros, then there is no need to parse
    pub is_empty: bool,

    #[deku(reader = "read_bds10(deku::input_bits, *is_empty)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds10: Option<DataLinkCapability>,

    #[deku(reader = "read_bds17(deku::input_bits, *is_empty)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds17: Option<GICBCapabilityReport>,

    #[deku(reader = "read_bds20(deku::input_bits, *is_empty)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds20: Option<AircraftIdentification>,

    #[deku(reader = "read_bds30(deku::input_bits, *is_empty)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds30: Option<ACASResolutionAdvisory>,

    #[deku(reader = "read_bds40(deku::input_bits, *is_empty)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds40: Option<SelectedVerticalIntention>,

    #[deku(reader = "read_bds44(deku::input_bits, *is_empty)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds44: Option<MeteorologicalRoutineAirReport>,

    #[deku(reader = "read_bds50(deku::input_bits, *is_empty)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds50: Option<TrackAndTurnReport>,

    #[deku(reader = "read_bds60(deku::input_bits, *is_empty)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds60: Option<HeadingAndSpeedReport>,
}

impl fmt::Display for DataSelector {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

fn read_bds10(
    input: &BitSlice<u8, Msb0>,
    empty: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<DataLinkCapability>), DekuError> {
    if empty {
        return Ok((input, None));
    }
    let (_, bytes, _) = input.domain().region().unwrap();

    if let Ok((_, bds10)) = DataLinkCapability::from_bytes((bytes, 0)) {
        Ok((input, Some(bds10)))
    } else {
        Ok((input, None))
    }
}

fn read_bds17(
    input: &BitSlice<u8, Msb0>,
    empty: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<GICBCapabilityReport>), DekuError> {
    if empty {
        return Ok((input, None));
    }
    let (_, bytes, _) = input.domain().region().unwrap();

    if let Ok((_, bds17)) = GICBCapabilityReport::from_bytes((bytes, 0)) {
        Ok((input, Some(bds17)))
    } else {
        Ok((input, None))
    }
}

fn read_bds20(
    input: &BitSlice<u8, Msb0>,
    empty: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<AircraftIdentification>), DekuError> {
    if empty {
        return Ok((input, None));
    }
    let (_, bytes, _) = input.domain().region().unwrap();

    if let Ok((_, bds20)) = AircraftIdentification::from_bytes((bytes, 0)) {
        Ok((input, Some(bds20)))
    } else {
        Ok((input, None))
    }
}

fn read_bds30(
    input: &BitSlice<u8, Msb0>,
    empty: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<ACASResolutionAdvisory>), DekuError> {
    if empty {
        return Ok((input, None));
    }
    let (_, bytes, _) = input.domain().region().unwrap();

    if let Ok((_, bds30)) = ACASResolutionAdvisory::from_bytes((bytes, 0)) {
        Ok((input, Some(bds30)))
    } else {
        Ok((input, None))
    }
}

fn read_bds40(
    input: &BitSlice<u8, Msb0>,
    empty: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<SelectedVerticalIntention>), DekuError>
{
    if empty {
        return Ok((input, None));
    }
    let (_, bytes, _) = input.domain().region().unwrap();
    if let Ok((_, bds40)) = SelectedVerticalIntention::from_bytes((bytes, 0)) {
        Ok((input, Some(bds40)))
    } else {
        Ok((input, None))
    }
}

fn read_bds44(
    input: &BitSlice<u8, Msb0>,
    empty: bool,
) -> Result<
    (&BitSlice<u8, Msb0>, Option<MeteorologicalRoutineAirReport>),
    DekuError,
> {
    if empty {
        return Ok((input, None));
    }
    let (_, bytes, _) = input.domain().region().unwrap();
    if let Ok((_, bds44)) =
        MeteorologicalRoutineAirReport::from_bytes((bytes, 0))
    {
        Ok((input, Some(bds44)))
    } else {
        Ok((input, None))
    }
}

fn read_bds50(
    input: &BitSlice<u8, Msb0>,
    empty: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<TrackAndTurnReport>), DekuError> {
    if empty {
        return Ok((input, None));
    }
    let (_, bytes, _) = input.domain().region().unwrap();
    if let Ok((_, bds50)) = TrackAndTurnReport::from_bytes((bytes, 0)) {
        Ok((input, Some(bds50)))
    } else {
        Ok((input, None))
    }
}

fn read_bds60(
    input: &BitSlice<u8, Msb0>,
    empty: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<HeadingAndSpeedReport>), DekuError> {
    if empty {
        return Ok((input, None));
    }
    let (_, bytes, _) = input.domain().region().unwrap();
    if let Ok((_, bds60)) = HeadingAndSpeedReport::from_bytes((bytes, 0)) {
        Ok((input, Some(bds60)))
    } else {
        Ok((input, None))
    }
}

fn check_empty_bds(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, bool), DekuError> {
    let mut inside_rest = rest;
    for _ in 0..=5 {
        let (for_rest, value) = u8::read(
            inside_rest,
            (deku::ctx::Endian::Big, deku::ctx::BitSize(8)),
        )?;
        if value != 0 {
            return Ok((rest, false));
        }
        inside_rest = for_rest;
    }
    Ok((rest, true))
}
