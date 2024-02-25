extern crate alloc;

use super::bds05::CPRFormat;
use alloc::fmt;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/*
 * +----+-----+---+-----+---+---+---------+---------+
 * | TC | MOV | S | TRK | T | F | LAT-CPR | LON-CPR |
 * +----+-----+---+-----+---+---+---------+---------+
 * | 5  |  7  | 1 |  7  | 1 | 1 |   17    |   17    |
 * +----+-----+---+-----+---+---+---------+---------+
 *  */

#[derive(Debug, PartialEq, DekuRead, Serialize, Copy, Clone)]
pub struct SurfacePosition {
    #[deku(bits = "5")]
    #[serde(skip)]
    pub tc: u8,

    #[deku(
        bits = "0",
        map = "|_val: u8| -> Result<_, DekuError> {Ok(14 - *tc)}"
    )]
    #[serde(rename = "NUCp")]
    pub nuc_p: u8,

    #[deku(reader = "read_groundspeed(deku::rest)")]
    pub groundspeed: Option<f64>,

    #[deku(bits = "1")] // bit 29
    #[serde(skip)]
    pub track_status: bool,

    #[deku(
        bits = "7",
        map = "|value: u8| -> Result<_, DekuError> {
            if *track_status {
                Ok(Some(value as f64 * 360. / 128.))
            } else { Ok(None) }
        }"
    )]
    pub track: Option<f64>,

    /// UTC sync or not
    #[deku(bits = "1")]
    #[serde(skip)]
    pub t: bool,

    pub odd_flag: CPRFormat,

    #[deku(bits = "17", endian = "big")]
    pub lat_cpr: u32,

    #[deku(bits = "17", endian = "big")]
    pub lon_cpr: u32,
}

fn read_groundspeed(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, mov) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(7)))?;
    let value = match mov {
        0 => None,
        1 => Some(0.),
        2..=8 => Some(0.125 + (mov - 2) as f64 * 0.125),
        9..=12 => Some(1. + (mov - 9) as f64 * 0.25),
        13..=38 => Some(2. + (mov - 13) as f64 * 0.25),
        39..=93 => Some(15. + (mov - 39) as f64 * 1.),
        94..=108 => Some(70. + (mov - 94) as f64 * 2.),
        109..=123 => Some(100. + (mov - 109) as f64 * 5.),
        124 => Some(175.),
        125..=u8::MAX => None, // Reserved
    };
    Ok((rest, value))
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum StatusForGroundTrack {
    Invalid = 0,
    Valid = 1,
}

impl fmt::Display for SurfacePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let groundspeed = self
            .groundspeed
            .map_or_else(|| "None".to_string(), |gs| format!("{gs} kts"));
        let track = self
            .track
            .map_or_else(|| "None".to_string(), |track| format!("{track}°"));
        writeln!(f, "  Groundspeed:   {}", groundspeed)?;
        writeln!(f, "  Track angle:   {}", track)?;
        writeln!(f, "  CPR odd flag:  {}", self.odd_flag)?;
        writeln!(f, "  CPR latitude:  ({})", self.lat_cpr)?;
        writeln!(f, "  CPR longitude: ({})", self.lon_cpr)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::adsb::ME::BDS06;
    use crate::decode::{Message, DF::ADSB};
    use hexlit::hex;

    #[test]
    fn test_surface_position() {
        let bytes = hex!("8c4841753a9a153237aef0f275be");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ADSB(adsb_msg) = msg.df {
            if let BDS06(SurfacePosition {
                track, groundspeed, ..
            }) = adsb_msg.message
            {
                assert_eq!(track, Some(92.8125));
                assert_eq!(groundspeed, Some(17.));
                return;
            }
        }
        unreachable!();
    }

    #[test]
    fn test_format() {
        let bytes = hex!("8c4841753a9a153237aef0f275be");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(
            format!("{msg}"),
            r#" DF17. Extended Squitter Surface position (BDS 0,6)
  Address:       484175 (Mode S / ADS-B)
  Groundspeed:   17 kts
  Track angle:   92.8125°
  CPR odd flag:  odd
  CPR latitude:  (39195)
  CPR longitude: (110320)
"#
        )
    }
}
