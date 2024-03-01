use super::f64_threedecimals;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Track and turn report (BDS 5,0)
 */
#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct TrackAndTurnReport {
    #[deku(reader = "read_roll(deku::rest)")] // 11 bits
    #[serde(rename = "roll", serialize_with = "f64_threedecimals")]
    // Roll angle (negative sign means left wing down)
    pub roll_angle: f64,

    #[deku(reader = "read_track(deku::rest)")] // 12 bits
    #[serde(rename = "track", serialize_with = "f64_threedecimals")]
    pub track_angle: f64,

    #[deku(reader = "read_groundspeed(deku::rest)")] // 11 bits
    /// Groundspeed in kts
    pub groundspeed: u16,

    #[deku(reader = "read_rate(deku::rest, *roll_angle)")] // 11 bits
    #[serde(serialize_with = "f64_threedecimals")]
    pub track_rate: f64,

    #[deku(reader = "read_tas(deku::rest, *groundspeed)")] // 11 bits
    #[serde(rename = "TAS")]
    /// True Airspeed (TAS) in kts, IAS is in BDS 0,6
    pub true_airspeed: u16,
}

fn read_roll(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, f64), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    if !status {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    let (rest, sign) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(9)))?;
    let roll = if sign > 0 {
        (value as f64 - 512.) * 45. / 256.
    } else {
        value as f64 * 45. / 256.
    };
    if roll.abs() > 50. {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    Ok((rest, roll))
}

fn read_track(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, f64), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    if !status {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    let (rest, sign) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    let value = if sign == 1 {
        value as i16 - 1024
    } else {
        value as i16
    };
    let mut track = value as f64 * 90. / 512.;
    if track < 0. {
        track += 360.
    }

    Ok((rest, track))
}

fn read_groundspeed(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, u16), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    if !status {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    let gs = value * 2;
    if gs > 600 {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    Ok((rest, gs))
}

fn read_rate(
    rest: &BitSlice<u8, Msb0>,
    roll: f64,
) -> Result<(&BitSlice<u8, Msb0>, f64), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    if !status {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    let (rest, sign) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(9)))?;

    if value == 0b111111111 {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }

    let value = if sign == 1 {
        value as i16 - 512
    } else {
        value as i16
    };
    let rate = value as f64 * 8. / 256.;

    if roll * rate < 0. {
        // signs must agree: left wing down = turn left
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }

    Ok((rest, rate))
}

fn read_tas(
    rest: &BitSlice<u8, Msb0>,
    gs: u16,
) -> Result<(&BitSlice<u8, Msb0>, u16), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    if !status {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    let tas = value * 2;

    if !(80..=500).contains(&tas) | ((gs as i16 - tas as i16).abs() > 200) {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    Ok((rest, tas))
}
