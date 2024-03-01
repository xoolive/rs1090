use super::op_f64_threedecimals;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
* ## Heading and speed report (BDS 6,0)
*
* - Barometric altitude rate
*   1. Air Data System
*   2. Inertial Reference System/Flight Management System
*
* - Inertial vertical velocity:
*   1. Flight Management Computer / GNSS integrated
*   2. Flight Management Computer (General)
*   3. Inertial Reference System/Flight Management System
*
*/
#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct HeadingAndSpeedReport {
    #[deku(reader = "read_heading(deku::rest)")] // 12 bits
    /// The magnetic heading is the aircraft's heading with respect to the magnetic North
    #[serde(
        rename = "heading",
        skip_serializing_if = "Option::is_none",
        serialize_with = "op_f64_threedecimals"
    )]
    pub magnetic_heading: Option<f64>,

    #[deku(reader = "read_ias(deku::rest)")] // 11 bits
    #[serde(rename = "IAS", skip_serializing_if = "Option::is_none")]
    /// Indicated Airspeed (IAS) in kts, TAS is in BDS 0,5
    pub indicated_airspeed: Option<u16>,

    #[deku(reader = "read_mach(deku::rest, *indicated_airspeed)")] // 11 bits
    #[serde(
        rename = "Mach",
        skip_serializing_if = "Option::is_none",
        serialize_with = "op_f64_threedecimals"
    )]
    /// Mach number
    pub mach_number: Option<f64>,

    #[deku(reader = "read_vertical(deku::rest)")] // 11 bits
    /// Barometric altitude rates (in ft/mn) are only derived from
    /// barometer measurements (noisy).
    #[serde(
        rename = "vrate_barometric",
        skip_serializing_if = "Option::is_none"
    )]
    pub barometric_altitude_rate: Option<i16>,

    #[deku(reader = "read_vertical(deku::rest)")] // 11 bits
    /// Inertial vertical velocities (in ft/mn) are values provided by
    /// navigational equipment from different sources including the FMS
    #[serde(
        rename = "vrate_inertial",
        skip_serializing_if = "Option::is_none"
    )]
    pub inertial_vertical_velocity: Option<i16>,
}

fn read_heading(
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
            return Err(DekuError::Assertion("BDS 6,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    let value = if sign == 1 {
        value as i16 - 1024
    } else {
        value as i16
    };
    let mut heading = value as f64 * 90. / 512.;
    if heading < 0. {
        heading += 360.
    }

    Ok((rest, Some(heading)))
}

fn read_ias(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u16>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 6,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    if (value == 0) | (value > 500) {
        return Err(DekuError::Assertion("BDS 6,0 status".to_string()));
    }
    Ok((rest, Some(value)))
}

fn read_mach(
    rest: &BitSlice<u8, Msb0>,
    ias: Option<u16>,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 6,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    let mach = value as f64 * 2.048 / 512.;

    if (mach == 0.) | (mach > 1.) {
        return Err(DekuError::Assertion("BDS 6,0 status".to_string()));
    }
    if let Some(ias) = ias {
        /*
         * >>> pitot.aero.cas2mach(250, 10000)
         * 0.45229071380275554
         *
         * Let's do this:
         * 10000 ft has IAS max to 250, i.e. Mach 0.45
         * forbid IAS > 250 and Mach < .5
         */
        if (ias > 250) & (mach < 0.4) {
            return Err(DekuError::Assertion("BDS 6,0 status".to_string()));
        }
        // this one is easy IAS = 150 (close to take-off) at FL 400 is Mach 0.5
        if (ias < 150) & (mach > 0.5) {
            return Err(DekuError::Assertion("BDS 6,0 status".to_string()));
        }
    }
    Ok((rest, Some(mach)))
}

fn read_vertical(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<i16>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, sign) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(9)))?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion("BDS 6,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    if (value == 0) | (value == 511) {
        // all zeros or all ones
        return Ok((rest, Some(0)));
    }
    let value = if sign == 1 {
        (value as i16 - 512) * 32
    } else {
        value as i16 * 32
    };

    if value.abs() > 6000 {
        Err(DekuError::Assertion("BDS 6,0 status".to_string()))
    } else {
        Ok((rest, Some(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_valid_bds60() {
        let bytes = hex!("a80004aaa74a072bfdefc1d5cb4f");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBIdentityReply { bds, .. } = msg.df {
            let HeadingAndSpeedReport {
                magnetic_heading,
                indicated_airspeed,
                mach_number,
                barometric_altitude_rate,
                inertial_vertical_velocity,
            } = bds.bds60.unwrap();
            assert_relative_eq!(
                magnetic_heading.unwrap(),
                110.391,
                max_relative = 1e-3
            );
            assert_eq!(indicated_airspeed.unwrap(), 259);
            assert_relative_eq!(mach_number.unwrap(), 0.7, max_relative = 1e-3);
            assert_eq!(barometric_altitude_rate.unwrap(), -2144);
            assert_eq!(inertial_vertical_velocity.unwrap(), -2016);
        } else {
            unreachable!();
        }
    }
    #[test]
    fn test_invalid_bds60() {
        let bytes = hex!("a0000638fa81c10000000081a92f");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(bds.bds60, None);
        } else {
            unreachable!();
        }
    }
}
