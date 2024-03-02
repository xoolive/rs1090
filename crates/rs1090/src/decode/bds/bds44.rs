use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Meteorological Routine Air Report (BDS 4,4)
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct MeteorologicalRoutineAirReport {
    /// Figure of merit / source
    #[deku(bits = 4)]
    #[serde(skip)]
    pub figure_of_merit: u8,

    #[deku(reader = "read_wind_speed(deku::rest)")]
    /// Wind speed in kts
    pub wind_speed: Option<u16>,
    #[deku(reader = "read_wind_direction(deku::rest, *wind_speed)")]
    /// Wind direction in degrees
    pub wind_direction: Option<f64>,

    #[deku(reader = "read_temperature(deku::rest)")]
    /// Static air temperature in Celsius (decoded with LSB=0,25)
    pub temperature: f64,

    #[deku(reader = "read_pressure(deku::rest)")]
    /// Average static pressure
    pub pressure: Option<u16>,

    #[deku(reader = "read_turbulence(deku::rest)")]
    /// Average static pressure
    pub turbulence: Option<Turbulence>,

    #[deku(reader = "read_humidity(deku::rest)")]
    /// Percentage of humidity
    pub humidity: Option<f64>,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum Turbulence {
    Nil,
    Light,
    Moderate,
    Severe,
}

fn read_wind_speed(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u16>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(9)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 4,4 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }
    if value > 250 {
        return Err(DekuError::Assertion("BDS 4,4 status".to_string()));
    }

    Ok((rest, Some(value)))
}

fn read_wind_direction(
    rest: &BitSlice<u8, Msb0>,
    speed: Option<u16>,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(9)))?;

    if speed.is_none() {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 4,4 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    Ok((rest, Some(value as f64 * 180. / 256.)))
}

fn read_temperature(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, f64), DekuError> {
    let (rest, sign) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))?;

    let temp = if sign == 1 {
        (value as f64 - 1024.) * 0.25
    } else {
        value as f64 * 0.25
    };

    if !(-80. ..=60.).contains(&temp) {
        return Err(DekuError::Assertion("BDS 4,4 status".to_string()));
    }
    Ok((rest, temp))
}

fn read_pressure(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u16>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(11)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 4,4 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    // Never seen any anyway
    Err(DekuError::Assertion("BDS 4,4 status".to_string()))

    // return Ok((rest, Some(value)));
}

fn read_turbulence(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<Turbulence>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(2)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 4,4 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    let value = match value {
        0 => Some(Turbulence::Nil),
        1 => Some(Turbulence::Light),
        2 => Some(Turbulence::Moderate),
        3 => Some(Turbulence::Severe),
        _ => None, // never happens anyway
    };

    Ok((rest, value))
}

fn read_humidity(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(6)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 4,4 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    Ok((rest, Some(value as f64 * 100. / 64.)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_valid_bds44() {
        let bytes = hex!("a0001692185bd5cf400000dfc696");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            let MeteorologicalRoutineAirReport {
                wind_speed,
                wind_direction,
                temperature,
                pressure,
                humidity,
                ..
            } = bds.bds44.unwrap();
            assert_eq!(wind_speed.unwrap(), 22);
            assert_relative_eq!(
                wind_direction.unwrap(),
                344.5,
                max_relative = 1e-3
            );
            assert_relative_eq!(temperature, -48.75, max_relative = 1e-3);
            assert_eq!(pressure, None);
            assert_eq!(humidity, None);
        } else {
            unreachable!();
        }
    }
}
