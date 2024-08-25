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
#[serde(tag = "bds", rename = "60")]
pub struct HeadingAndSpeedReport {
    #[deku(reader = "read_heading(deku::reader)")] // 12 bits
    /// The magnetic heading is the aircraft's heading with respect to the magnetic North
    #[serde(rename = "heading", skip_serializing_if = "Option::is_none")]
    pub magnetic_heading: Option<f64>,

    #[deku(reader = "read_ias(deku::reader)")] // 11 bits
    #[serde(rename = "IAS", skip_serializing_if = "Option::is_none")]
    /// Indicated Airspeed (IAS) in kts, TAS is in BDS 0,5
    pub indicated_airspeed: Option<u16>,

    #[deku(reader = "read_mach(deku::reader, *indicated_airspeed)")] // 11 bits
    #[serde(rename = "Mach", skip_serializing_if = "Option::is_none")]
    /// Mach number
    pub mach_number: Option<f64>,

    #[deku(reader = "read_vertical(deku::reader)")] // 11 bits
    /// Barometric altitude rates (in ft/mn) are only derived from
    /// barometer measurements (noisy).
    #[serde(
        rename = "vrate_barometric",
        skip_serializing_if = "Option::is_none"
    )]
    pub barometric_altitude_rate: Option<i16>,

    #[deku(reader = "read_vertical(deku::reader)")] // 11 bits
    /// Inertial vertical velocities (in ft/mn) are values provided by
    /// navigational equipment from different sources including the FMS
    #[serde(
        rename = "vrate_inertial",
        skip_serializing_if = "Option::is_none"
    )]
    pub inertial_vertical_velocity: Option<i16>,
}

fn read_heading<R: std::io::Read>(
    reader: &mut Reader<R>,
) -> Result<Option<f64>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let sign = u8::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(10)),
    )?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion("BDS 6,0 status".into()));
        } else {
            return Ok(None);
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

    Ok(Some(heading))
}

fn read_ias<R: std::io::Read>(
    reader: &mut Reader<R>,
) -> Result<Option<u16>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(10)),
    )?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 6,0 status".into()));
        } else {
            return Ok(None);
        }
    }

    if (value == 0) | (value > 500) {
        return Err(DekuError::Assertion("BDS 6,0 status".into()));
    }
    Ok(Some(value))
}

fn read_mach<R: std::io::Read>(
    reader: &mut Reader<R>,
    ias: Option<u16>,
) -> Result<Option<f64>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(10)),
    )?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 6,0 status".into()));
        } else {
            return Ok(None);
        }
    }

    let mach = value as f64 * 2.048 / 512.;

    if (mach == 0.) | (mach > 1.) {
        return Err(DekuError::Assertion("BDS 6,0 status".into()));
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
            return Err(DekuError::Assertion("BDS 6,0 status".into()));
        }
        // this one is easy IAS = 150 (close to take-off) at FL 400 is Mach 0.5
        if (ias < 150) & (mach > 0.5) {
            return Err(DekuError::Assertion("BDS 6,0 status".into()));
        }
    }
    Ok(Some(mach))
}

fn read_vertical<R: std::io::Read>(
    reader: &mut Reader<R>,
) -> Result<Option<i16>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let sign = u8::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(9)),
    )?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion("BDS 6,0 status".into()));
        } else {
            return Ok(None);
        }
    }

    if (value == 0) | (value == 511) {
        // all zeros or all ones
        return Ok(Some(0));
    }
    let value = if sign == 1 {
        (value as i16 - 512) * 32
    } else {
        value as i16 * 32
    };

    if value.abs() > 6000 {
        Err(DekuError::Assertion("BDS 6,0 status".into()))
    } else {
        Ok(Some(value))
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
