use super::op_f64_threedecimals;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Track and turn report (BDS 5,0)
 */
#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct TrackAndTurnReport {
    #[deku(reader = "read_roll(deku::rest)")] // 11 bits
    #[serde(rename = "roll", serialize_with = "op_f64_threedecimals")]
    // Roll angle (negative sign means left wing down)
    pub roll_angle: Option<f64>,

    #[deku(reader = "read_track(deku::rest)")] // 12 bits
    #[serde(rename = "track", serialize_with = "op_f64_threedecimals")]
    pub track_angle: Option<f64>,

    #[deku(reader = "read_groundspeed(deku::rest)")] // 11 bits
    /// Groundspeed in kts
    pub groundspeed: Option<u16>,

    #[deku(reader = "read_rate(deku::rest, *roll_angle)")] // 11 bits
    #[serde(serialize_with = "op_f64_threedecimals")]
    pub track_rate: Option<f64>,

    #[deku(reader = "read_tas(deku::rest, *groundspeed)")] // 11 bits
    #[serde(rename = "TAS")]
    /// True Airspeed (TAS) in kts, IAS is in BDS 0,6
    pub true_airspeed: Option<u16>,
}

fn read_roll(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, sign) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(9)))?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    let roll = if sign > 0 {
        (value as f64 - 512.) * 45. / 256.
    } else {
        value as f64 * 45. / 256.
    };
    if roll.abs() > 50. {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    Ok((rest, Some(roll)))
}

fn read_track(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, sign) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    let value = if sign == 1 {
        value as i16 - 1024
    } else {
        value as i16
    };
    let mut track = value as f64 * 90. / 512.;
    if track < 0. {
        track += 360.
    }

    Ok((rest, Some(track)))
}

fn read_groundspeed(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u16>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    let gs = value * 2;
    if gs > 600 {
        return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
    }
    Ok((rest, Some(gs)))
}

fn read_rate(
    rest: &BitSlice<u8, Msb0>,
    roll: Option<f64>,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, sign) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(9)))?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    if value == 0b111111111 {
        return Ok((rest, None));
    }

    let value = if sign == 1 {
        value as i16 - 512
    } else {
        value as i16
    };
    let rate = value as f64 * 8. / 256.;

    if let Some(roll) = roll {
        if roll * rate < 0. {
            // signs must agree: left wing down = turn left
            return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
        }
    }

    Ok((rest, Some(rate)))
}

fn read_tas(
    rest: &BitSlice<u8, Msb0>,
    gs: Option<u16>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u16>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    let tas = value * 2;

    if let Some(gs) = gs {
        if !(80..=500).contains(&tas) | ((gs as i16 - tas as i16).abs() > 200) {
            return Err(DekuError::Assertion("BDS 5,0 status".to_string()));
        }
    }
    Ok((rest, Some(tas)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_valid_bds50() {
        let bytes = hex!("a000139381951536e024d4ccf6b5");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            let TrackAndTurnReport {
                roll_angle,
                track_angle,
                true_airspeed,
                groundspeed,
                track_rate,
            } = bds.bds50.unwrap();
            assert_relative_eq!(roll_angle.unwrap(), 2.1, max_relative = 1e-2);
            assert_relative_eq!(
                track_angle.unwrap(),
                114.258,
                max_relative = 1e-3
            );
            assert_eq!(groundspeed.unwrap(), 438);
            assert_eq!(true_airspeed.unwrap(), 424);
            assert_relative_eq!(
                track_rate.unwrap(),
                0.125,
                max_relative = 1e-3
            );
        } else {
            unreachable!();
        }
    }
    #[test]
    fn test_invalid_bds50() {
        let bytes = hex!("a0000638fa81c10000000081a92f");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(bds.bds50, None);
        } else {
            unreachable!();
        }
    }
}
