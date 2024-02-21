extern crate alloc;

use super::bds05::CPRFormat;
use alloc::fmt;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::ser::{Serialize, SerializeStruct, Serializer};

/*
 * +----+-----+---+-----+---+---+---------+---------+
 * | TC | MOV | S | TRK | T | F | LAT-CPR | LON-CPR |
 * +----+-----+---+-----+---+---+---------+---------+
 * | 5  |  7  | 1 |  7  | 1 | 1 |   17    |   17    |
 * +----+-----+---+-----+---+---+---------+---------+
 *  */

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
pub struct SurfacePosition {
    #[deku(bits = "5")]
    pub tc: u8, // NUCp = 14 - tc
    #[deku(
        bits = "7",
        map = "|mov: u8| -> Result<_, DekuError> { Ok(read_groundspeed(mov)) }"
    )]
    pub groundspeed: Option<f64>,
    pub status: StatusForGroundTrack,
    #[deku(reader = "read_track(deku::rest, *status)")]
    pub track: Option<f64>,
    /// UTC sync or not
    #[deku(bits = "1")]
    pub t: bool,
    pub odd_flag: CPRFormat,
    #[deku(bits = "17", endian = "big")]
    pub lat_cpr: u32,
    #[deku(bits = "17", endian = "big")]
    pub lon_cpr: u32,
}

impl Serialize for SurfacePosition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Message", 6)?;
        state.serialize_field("NUCp", &(14 - self.tc))?;
        state.serialize_field("groundspeed", &self.groundspeed)?;
        state.serialize_field("track", &self.track)?;
        let flag = match self.odd_flag {
            CPRFormat::Odd => "odd",
            CPRFormat::Even => "even",
        };
        state.serialize_field("odd_flag", &flag)?;
        state.serialize_field("lat_cpr", &self.lat_cpr)?;
        state.serialize_field("lon_cpr", &self.lon_cpr)?;

        state.end()
    }
}

fn read_track(
    rest: &BitSlice<u8, Msb0>,
    status: StatusForGroundTrack,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, value) =
        u8::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(7)))?;
    let track = match status {
        StatusForGroundTrack::Invalid => None,
        StatusForGroundTrack::Valid => Some(value as f64 * 360. / 128.),
    };
    Ok((rest, track))
}
fn read_groundspeed(mov: u8) -> Option<f64> {
    match mov {
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
    }
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
    use crate::decode::adsb::Typecode::SurfacePosition;
    use crate::decode::{Message, DF::ADSB};
    use hexlit::hex;

    #[test]
    fn test_surface_position() {
        let bytes = hex!("8c4841753a9a153237aef0f275be");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ADSB(adsb_msg) = msg.df {
            if let SurfacePosition(sp) = adsb_msg.message {
                assert_eq!(sp.status, StatusForGroundTrack::Valid);
                assert_eq!(sp.track, Some(92.8125));
                assert_eq!(sp.groundspeed, Some(17.));
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
