use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Meteorological Hazard Report (BDS 4,5)
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[serde(tag = "bds", rename = "45")]
pub struct MeteorologicalHazardReport {
    #[deku(reader = "read_level(deku::rest)")]
    /// Turbulence level
    pub turbulence: Option<Level>,

    #[deku(reader = "read_level(deku::rest)")]
    /// Wind shear
    pub wind_shear: Option<Level>,

    #[deku(reader = "read_level(deku::rest)")]
    /// Icing
    pub icing: Option<Level>,

    #[deku(reader = "read_level(deku::rest)")]
    /// Wake vortex
    pub wake_vortex: Option<Level>,

    #[deku(reader = "read_temperature(deku::rest)")]
    /// Static air temperature (in °C)
    pub static_temperature: f64,

    #[deku(reader = "read_pressure(deku::rest)")]
    /// Average static pressure (in hPa)
    pub static_pressure: Option<u32>,

    #[deku(reader = "read_height(deku::rest)")]
    /// Radio height (in ft)
    pub radio_height: Option<u32>,

    #[deku(bits = "8", map = "fail_if_not_zero")]
    #[serde(skip)]
    pub reserved: u8,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum Level {
    Nil,
    Light,
    Moderate,
    Severe,
}

fn read_level(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<Level>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(2)))?;

    match (status, value) {
        (true, 0) => Ok((rest, Some(Level::Nil))),
        (true, 1) => Ok((rest, Some(Level::Light))),
        (true, 2) => Ok((rest, Some(Level::Moderate))),
        (true, 3) => Ok((rest, Some(Level::Severe))),
        (true, _) => unreachable!(),
        (false, 0) => Ok((rest, None)),
        (false, _) => Err(DekuError::Assertion("invalid data".to_string())),
    }
}

fn read_temperature(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, f64), DekuError> {
    let (rest, sign) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(9)))?;

    let temperature = match sign {
        true => (value as f64 - 512.) * 0.25,
        false => value as f64 * 0.25,
    };

    if !(-80. ..=60.).contains(&temperature) {
        return Err(DekuError::Assertion(
            "Static temperature between -80 and +60".to_string(),
        ));
    }
    Ok((rest, temperature))
}

fn read_pressure(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u32>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u32::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(11)))?;

    match (status, value) {
        (true, value) => Ok((rest, Some(value))),
        (false, _) => Ok((rest, None)),
        // In practice, I see quite some pressure fields with invalid status but non zero values
        // (false, 0) => Ok((rest, None)),
        // (false, _) => Err(DekuError::Assertion("invalid data".to_string())),
    }
}

fn read_height(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u32>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u32::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(12)))?;

    match (status, value) {
        (true, value) => Ok((rest, Some(value * 16))),
        (false, 0) => Ok((rest, None)),
        (false, _) => Err(DekuError::Assertion("invalid data".to_string())),
    }
}

fn fail_if_not_zero(value: u8) -> Result<u8, DekuError> {
    if value == 0 {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "Reserved bits must be zero".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_valid_bds44() {
        let bytes = hex!("a00004190001fb80000000000000");
        // let bytes = hex!("a00005b30001f940000000000000");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            let MeteorologicalHazardReport {
                turbulence,
                wind_shear,
                icing,
                wake_vortex,
                static_temperature,
                static_pressure,
                radio_height,
                ..
            } = bds.bds45.unwrap();
            assert_eq!(turbulence, None);
            assert_eq!(wind_shear, None);
            assert_eq!(icing, None);
            assert_eq!(wake_vortex, None);
            assert_eq!(static_temperature, 31.5);
            assert_eq!(static_pressure, Some(1536));
            assert_eq!(radio_height, None);
        } else {
            unreachable!();
        }
    }
}
