extern crate alloc;

use crate::decode::{decode_id13, gray2alt};
use alloc::fmt;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * +----+----+-----+-----+---+---+---------+---------+
 * | TC | SS | SAF | ALT | T | F | LAT-CPR | LON-CPR |
 * +----+----+-----+-----+---+---+---------+---------+
 * | 5  | 2  |  1  | 12  | 1 | 1 |   17    |   17    |
 * +----+----+-----+-----+---+---+---------+---------+
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct PositionAltitude {
    #[deku(bits = "5")]
    #[serde(skip)]
    pub tc: u8, // NUCp = 18 - tc

    #[deku(
        bits = "0",
        map = "|_val: u8| -> Result<_, DekuError> {Ok(18 - *tc)}"
    )]
    #[serde(rename = "NUCp")]
    pub nuc_p: u8,

    #[serde(skip)]
    pub ss: SurveillanceStatus,

    // TODO SAF in v0 and v1, NICb in v2
    #[deku(
        bits = "1",
        map = "|v| -> Result<_, DekuError> {
            if *tc<19 {Ok(Some(v))} else {Ok(None)}
        }"
    )]
    #[serde(rename = "NICb", skip_serializing_if = "Option::is_none")]
    pub saf_or_nicb: Option<u8>,

    #[deku(reader = "decode_ac12(deku::rest)")]
    #[serde(rename = "altitude")]
    pub alt: Option<u16>,

    #[deku(reader = "read_source(deku::rest, *tc)")]
    pub source: Source,

    /// UTC sync or not
    #[deku(bits = "1")]
    #[serde(skip)]
    pub t: bool,

    /// Odd or even
    pub odd_flag: CPRFormat,

    #[deku(bits = "17", endian = "big")]
    pub lat_cpr: u32,

    #[deku(bits = "17", endian = "big")]
    pub lon_cpr: u32,
}

fn decode_ac12(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u16>), DekuError> {
    let (rest, num) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(12)))?;

    let q = num & 0x10;

    if q > 0 {
        let n = ((num & 0x0fe0) >> 1) | (num & 0x000f);
        let n = n * 25;
        if n > 1000 {
            Ok((rest, u16::try_from(n - 1000).ok()))
        } else {
            Ok((rest, None))
        }
    } else {
        let mut n = ((num & 0x0fc0) << 1) | (num & 0x003f);
        n = decode_id13(n);
        if let Ok(n) = gray2alt(n) {
            Ok((rest, u16::try_from(n * 100).ok()))
        } else {
            Ok((rest, None))
        }
    }
}

fn read_source(
    rest: &BitSlice<u8, Msb0>,
    tc: u8,
) -> Result<(&BitSlice<u8, Msb0>, Source), DekuError> {
    let source = if tc < 19 {
        Source::Barometric
    } else {
        Source::Gnss
    };
    Ok((rest, source))
}

impl fmt::Display for PositionAltitude {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let altitude = self.alt.map_or_else(
            || "None".to_string(),
            |altitude| format!("{altitude} ft"),
        );
        writeln!(f, "  Altitude:      {} {}", altitude, self.source)?;
        writeln!(f, "  CPR type:      Airborne")?;
        writeln!(f, "  CPR odd flag:  {}", self.odd_flag)?;
        writeln!(f, "  CPR latitude:  ({})", self.lat_cpr)?;
        writeln!(f, "  CPR longitude: ({})", self.lon_cpr)?;
        Ok(())
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

/// Even / Odd
#[derive(Debug, PartialEq, Eq, Serialize, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
#[serde(rename_all = "snake_case")]
pub enum CPRFormat {
    Even = 0,
    Odd = 1,
}

#[derive(Debug, PartialEq, Eq, Serialize, Copy, Clone)]
pub enum Source {
    #[serde(rename = "barometric")]
    Barometric = 0,
    #[serde(rename = "GNSS")]
    Gnss = 1,
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

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Barometric => "barometric",
                Self::Gnss => "GNSS",
            }
        )
    }
}
