extern crate alloc;

use super::bds05::CPRFormat;
use alloc::fmt;
use deku::prelude::*;

/*
 * +----+-----+---+-----+---+---+---------+---------+
 * | TC | MOV | S | TRK | T | F | LAT-CPR | LON-CPR |
 * +----+-----+---+-----+---+---+---------+---------+
 * | 5  |  7  | 1 |  7  | 1 | 1 |   17    |   17    |
 * +----+-----+---+-----+---+---+---------+---------+
 *  */

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
pub struct SurfacePosition {
    #[deku(bits = "5")]
    pub tc: u8, // NUCp = 14 - tc
    #[deku(bits = "7")]
    pub mov: u8,
    pub status: StatusForGroundTrack,
    #[deku(bits = "7")]
    pub trk: u8,
    /// UTC sync or not
    #[deku(bits = "1")]
    pub t: bool,
    pub odd_flag: CPRFormat,
    #[deku(bits = "17", endian = "big")]
    pub lat_cpr: u32,
    #[deku(bits = "17", endian = "big")]
    pub lon_cpr: u32,
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum StatusForGroundTrack {
    Invalid = 0,
    Valid = 1,
}

impl SurfacePosition {
    pub fn read_track(&self) -> Option<f64> {
        match self.status {
            StatusForGroundTrack::Invalid => None,
            StatusForGroundTrack::Valid => Some(self.trk as f64 * 360. / 128.),
        }
    }
    pub fn read_groundspeed(&self) -> Option<f64> {
        match self.mov {
            0 => None,
            1 => Some(0.),
            2..=8 => Some(0.125 + (self.mov - 2) as f64 * 0.125),
            9..=12 => Some(1. + (self.mov - 9) as f64 * 0.25),
            13..=38 => Some(2. + (self.mov - 13) as f64 * 0.25),
            39..=93 => Some(15. + (self.mov - 39) as f64 * 1.),
            94..=108 => Some(70. + (self.mov - 94) as f64 * 2.),
            109..=123 => Some(100. + (self.mov - 109) as f64 * 5.),
            124 => Some(175.),
            125..=u8::MAX => None, // Reserved
        }
    }
}

impl fmt::Display for SurfacePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let groundspeed = self
            .read_groundspeed()
            .map_or_else(|| "None".to_string(), |gs| format!("{gs} kts"));
        let track = self
            .read_track()
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
                assert_eq!(sp.read_track(), Some(92.8125));
                assert_eq!(sp.read_groundspeed(), Some(17.));
                return;
            }
        }
        unreachable!();
    }
}
