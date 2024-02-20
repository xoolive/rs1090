extern crate alloc;

use super::{decode_id13_field, mode_a_to_mode_c};
use alloc::fmt;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;

/**
 * +----+----+-----+-----+---+---+---------+---------+
 * | TC | SS | SAF | ALT | T | F | LAT-CPR | LON-CPR |
 * +----+----+-----+-----+---+---+---------+---------+
 * | 5  | 2  |  1  | 12  | 1 | 1 |   17    |   17    |
 * +----+----+-----+-----+---+---+---------+---------+
 */

#[derive(Debug, PartialEq, Eq, DekuRead, Default, Copy, Clone)]
pub struct PositionAltitude {
    #[deku(bits = "5")]
    pub tc: u8, // NUCp = 18 - tc
    pub ss: SurveillanceStatus,
    #[deku(bits = "1")]
    pub saf_or_nicb: u8, // SAF in v0 and v1, NICb in v2
    #[deku(reader = "Self::read(deku::rest)")]
    pub alt: Option<u16>,
    /// UTC sync or not
    #[deku(bits = "1")]
    pub t: bool,
    /// Odd or even
    pub odd_flag: CPRFormat,
    #[deku(bits = "17", endian = "big")]
    pub lat_cpr: u32,
    #[deku(bits = "17", endian = "big")]
    pub lon_cpr: u32,
}

impl fmt::Display for PositionAltitude {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let altitude = self.alt.map_or_else(
            || "None".to_string(),
            |altitude| format!("{altitude} ft barometric"),
        );
        writeln!(f, "  Altitude:      {altitude}")?;
        writeln!(f, "  CPR type:      Airborne")?;
        writeln!(f, "  CPR odd flag:  {}", self.odd_flag)?;
        writeln!(f, "  CPR latitude:  ({})", self.lat_cpr)?;
        writeln!(f, "  CPR longitude: ({})", self.lon_cpr)?;
        Ok(())
    }
}

impl PositionAltitude {
    /// `decodeAC12Field`
    fn read(rest: &BitSlice<u8, Msb0>) -> Result<(&BitSlice<u8, Msb0>, Option<u16>), DekuError> {
        let (rest, num) = u32::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(12)))?;

        let q = num & 0x10;

        if q > 0 {
            let n = ((num & 0x0fe0) >> 1) | (num & 0x000f);
            let n = n * 25;
            if n > 1000 {
                // TODO: maybe replace with Result->Option
                Ok((rest, u16::try_from(n - 1000).ok()))
            } else {
                Ok((rest, None))
            }
        } else {
            let mut n = ((num & 0x0fc0) << 1) | (num & 0x003f);
            n = decode_id13_field(n);
            if let Ok(n) = mode_a_to_mode_c(n) {
                Ok((rest, u16::try_from(n * 100).ok()))
            } else {
                Ok((rest, None))
            }
        }
    }
}

/// SPI Condition
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "2")]
pub enum SurveillanceStatus {
    NoCondition = 0,
    PermanentAlert = 1,
    TemporaryAlert = 2,
    SPICondition = 3,
}

impl Default for SurveillanceStatus {
    fn default() -> Self {
        Self::NoCondition
    }
}

/// Even / Odd
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum CPRFormat {
    Even = 0,
    Odd = 1,
}

impl Default for CPRFormat {
    fn default() -> Self {
        Self::Even
    }
}

impl fmt::Display for CPRFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Even => "even",
                Self::Odd => "odd",
            }
        )
    }
}
