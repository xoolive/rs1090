#![allow(clippy::suspicious_else_formatting)]

use super::super::cpr::CPRFormat;
use deku::prelude::*;
use serde::Serialize;
use std::fmt;
use tracing::debug;

/**
 * ## Surface Position (BDS 0,6)
 *
 * When an aircraft is on the ground, a different type of message is used to
 * broadcast its position information. Unlike the airborne position message, the
 * surface position message also includes the speed of the aircraft. Since no
 * altitude information needs to be transmitted, this provides some extra bits
 * for more information, such as speed and track angle.
 *
 *
 * | TC  | MOV | S   | TRK | T   | F   | LAT-CPR | LON-CPR |
 * | --- | --- | --- | --- | --- | --- | ------- | ------- |
 * | 5   | 7   | 1   | 7   | 1   | 1   | 17      | 17      |
 *
 */

#[derive(Debug, PartialEq, DekuRead, Serialize, Copy, Clone)]
pub struct SurfacePosition {
    #[deku(bits = 5)]
    pub tc: u8,

    #[deku(skip, default = "14 - tc")]
    #[serde(rename = "NUCp")]
    /// Navigation Uncertainty Category (position), based on the typecode
    pub nuc_p: u8,

    #[deku(reader = "read_groundspeed(deku::reader)")]
    /// The groundspeed in kts, None if not available
    pub groundspeed: Option<f64>,

    #[deku(bits = "1")] // bit 29
    #[serde(skip)]
    /// A flag stating whether the ground track is available
    pub track_status: bool,

    #[deku(
        bits = "7",
        map = "|value: u8| -> Result<_, DekuError> {
            if *track_status {
                Ok(Some(value as f64 * 360. / 128.))
            } else {
                Ok(None)
            }
        }"
    )]
    /// The track angle in degrees, relative to the true North, None if not available
    pub track: Option<f64>,

    // UTC sync or not
    #[deku(bits = "1")]
    #[serde(skip)]
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

/**
 * The movement field encodes the aircraft ground speed. The ground speed of the
 * aircraft is encoded non-linearly and with different quantizations. This is to
 * ensure that a lower speed can be encoded with a improved precision than a
 * higher speed.
 */
fn read_groundspeed<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<f64>, DekuError> {
    let mov = u8::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(7)),
    )?;
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
    debug!("Groundspeed value: {:?}", value);
    Ok(value)
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "1")]
pub enum StatusForGroundTrack {
    Invalid = 0,
    Valid = 1,
}

impl fmt::Display for SurfacePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Surface position (BDS 0,6)")?;
        let groundspeed = self
            .groundspeed
            .map_or_else(|| "None".to_string(), |gs| format!("{gs} kts"));
        let track = self
            .track
            .map_or_else(|| "None".to_string(), |track| format!("{track}°"));
        writeln!(f, "  Groundspeed:   {}", groundspeed)?;
        writeln!(f, "  Track angle:   {}", track)?;
        writeln!(f, "  CPR parity:    {}", self.parity)?;
        writeln!(f, "  CPR latitude:  ({})", self.lat_cpr)?;
        writeln!(f, "  CPR longitude: ({})", self.lon_cpr)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_surface_position() {
        tracing_subscriber::fmt::init();
        let bytes = hex!("8c4841753a9a153237aef0f275be");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            if let ME::BDS06(SurfacePosition {
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
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        assert_eq!(
            format!("{msg}"),
            r#" DF17. Extended Squitter
  Address:       484175
  Air/Ground:    ground
  Surface position (BDS 0,6)
  Groundspeed:   17 kts
  Track angle:   92.8125°
  CPR parity:    odd
  CPR latitude:  (39195)
  CPR longitude: (110320)
"#
        )
    }
}
