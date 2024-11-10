use deku::prelude::*;
use serde::Serialize;
use tracing::trace;

/**
 * ## Meteorological Hazard Report (BDS 4,5)
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[serde(tag = "bds", rename = "45")]
pub struct MeteorologicalHazardReport {
    #[deku(reader = "read_level(deku::reader)")]
    /// Turbulence level
    pub turbulence: Option<Level>,

    #[deku(reader = "read_level(deku::reader)")]
    /// Wind shear
    pub wind_shear: Option<Level>,

    #[deku(reader = "read_level(deku::reader)")]
    /// Microburst
    pub microburst: Option<Level>,

    #[deku(reader = "read_level(deku::reader)")]
    /// Icing
    pub icing: Option<Level>,

    #[deku(reader = "read_level(deku::reader)")]
    /// Wake vortex
    pub wake_vortex: Option<Level>,

    #[deku(reader = "read_temperature(deku::reader)")]
    /// Static air temperature (in °C)
    pub static_temperature: Option<f64>,

    #[deku(reader = "read_pressure(deku::reader)")]
    /// Average static pressure (in hPa)
    pub static_pressure: Option<u32>,

    #[deku(reader = "read_height(deku::reader)")]
    /// Radio height (in ft)
    pub radio_height: Option<u32>,

    #[deku(bits = "5", map = "fail_if_not_zero")]
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

fn read_level<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<Level>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u8::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(2)),
    )?;

    trace!("Reading status {} value {}", status, value);

    match (status, value) {
        (true, 0) => Ok(Some(Level::Nil)),
        (true, 1) => Ok(Some(Level::Light)),
        (true, 2) => Ok(Some(Level::Moderate)),
        (true, 3) => Ok(Some(Level::Severe)),
        (true, _) => unreachable!(),
        (false, 0) => Ok(None),
        (false, _) => Err(DekuError::Assertion("invalid data".into())),
    }
}

fn read_temperature<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<f64>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let sign = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(9)),
    )?;

    let temperature = match sign {
        true => (value as f64 - 512.) * 0.25,
        false => value as f64 * 0.25,
    };

    trace!(
        "Reading temperature status {} value {}",
        status,
        temperature
    );

    match (status, value, temperature) {
        (true, _, temperature) if (-80. ..=60.).contains(&temperature) => {
            Ok(Some(temperature))
        }
        (true, _, _) => Err(DekuError::Assertion(
            "Temperature {} should be between -80 and +60".into(),
        )),
        //(false, _) => Ok(None),
        // In practice, I see quite some pressure fields with invalid status but non zero values
        (false, 0, _) => Ok(None),
        (false, _, _) => Err(DekuError::Assertion("invalid data".into())),
    }
}

fn read_pressure<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<u32>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u32::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(11)),
    )?;

    trace!("Reading pressure status {} value {}", status, value);

    match (status, value) {
        (true, value) => Ok(Some(value)),
        //(false, _) => Ok(None),
        // In practice, I see quite some pressure fields with invalid status but non zero values
        (false, 0) => Ok(None),
        (false, _) => Err(DekuError::Assertion("invalid data".into())),
    }
}

fn read_height<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<u32>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u32::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(12)),
    )?;

    trace!("Reading height status {} value {}", status, value);

    match (status, value) {
        (true, value) => Ok(Some(value * 16)),
        (false, 0) => Ok(None),
        (false, _) => Err(DekuError::Assertion("invalid data".into())),
    }
}

fn fail_if_not_zero(value: u8) -> Result<u8, DekuError> {
    if value == 0 {
        Ok(value)
    } else {
        Err(DekuError::Assertion("Reserved bits must be zero".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_valid_bds45() {
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
            assert_eq!(static_temperature, Some(-4.5));
            assert_eq!(static_pressure, None);
            assert_eq!(radio_height, None);
        } else {
            unreachable!();
        }
    }
}
