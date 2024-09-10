use crate::decode::cpr::CPRFormat;
use crate::decode::{decode_id13, gray2alt};
use deku::prelude::*;
use serde::Serialize;
use std::fmt;

/**
 * ## Airborne Position (BDS 0,5)
 *
 * with barometric altitude (TC=9..=18) or geometric height (TC=20..=22)
 *
 * | TC | SS | SAF | ALT | T | F | LAT-CPR | LON-CPR |
 * | -- | -- | --- | --- | - | - | ------- | ------- |
 * | 5  | 2  |  1  | 12  | 1 | 1 |   17    |   17    |
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct AirbornePosition {
    #[deku(bits = 5)]
    tc: u8,

    #[deku(
        skip,
        default = "
        match *tc {
            n if n < 19 => 18 - tc,
            20 | 21 => 29 - tc,
            _ => 0
        }
        "
    )]
    #[serde(rename = "NUCp")]
    /// The Navigation Uncertainty Category Position (NUCp)
    /// (directly based on the typecode)
    pub nuc_p: u8,

    #[serde(skip)]
    /// Decode the surveillance status
    pub ss: SurveillanceStatus,

    #[deku(
        bits = "1",
        map = "|v| -> Result<_, DekuError> {
            if *tc < 19 { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(rename = "NICb", skip_serializing_if = "Option::is_none")]
    /// Single Antenna Flag in ADSB v0 or v1,
    /// Navigation Integrity Category Supplement-b (NICb) in ADSB v2
    pub saf_or_nicb: Option<u8>,

    #[deku(reader = "decode_ac12(deku::reader)")]
    #[serde(rename = "altitude")]
    /// Decode the altitude in feet, encoded on 12 bits.
    /// None if not available.
    pub alt: Option<u16>,

    #[deku(reader = "read_source(*tc)")]
    /// Decode the altitude source (GNSS or barometric),
    /// most commonly equal to barometric
    pub source: Source,

    #[deku(bits = "1")]
    #[serde(skip)]
    // UTC sync or not
    pub t: bool,

    pub parity: CPRFormat,

    #[deku(bits = "17", endian = "big")]
    pub lat_cpr: u32,

    #[deku(bits = "17", endian = "big")]
    pub lon_cpr: u32,

    #[deku(skip, default = "None")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,

    #[deku(skip, default = "None")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

/// Decode altitude value encoded on 12 bits
fn decode_ac12<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<u16>, DekuError> {
    let num = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(12)),
    )?;

    let q = num & 0x10;

    if q > 0 {
        let n = ((num & 0x0fe0) >> 1) | (num & 0x000f);
        let n = n * 25;
        if n > 1000 {
            Ok(Some(n - 1000))
        } else {
            Ok(None)
        }
    } else {
        let mut n = ((num & 0x0fc0) << 1) | (num & 0x003f);
        n = decode_id13(n);
        if let Ok(n) = gray2alt(n) {
            Ok(u16::try_from(n * 100).ok())
        } else {
            Ok(None)
        }
    }
}

fn read_source(tc: u8) -> Result<Source, DekuError> {
    let source = if tc < 19 {
        Source::Barometric
    } else {
        Source::Gnss
    };
    Ok(source)
}

impl fmt::Display for AirbornePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  AirbornePosition (BDS 0,5)")?;
        let altitude = self.alt.map_or_else(
            || "None".to_string(),
            |altitude| format!("{altitude} ft"),
        );
        writeln!(f, "  Altitude:      {} {}", altitude, self.source)?;
        writeln!(f, "  CPR type:      Airborne")?;
        writeln!(f, "  CPR parity:    {}", self.parity)?;
        writeln!(f, "  CPR latitude:  ({})", self.lat_cpr)?;
        writeln!(f, "  CPR longitude: ({})", self.lon_cpr)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "2")]
pub enum SurveillanceStatus {
    NoCondition = 0,
    PermanentAlert = 1,
    TemporaryAlert = 2,
    SPICondition = 3,
}

#[derive(Debug, PartialEq, Eq, Serialize, Copy, Clone)]
pub enum Source {
    #[serde(rename = "barometric")]
    Barometric = 0,
    #[serde(rename = "GNSS")]
    Gnss = 1,
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
