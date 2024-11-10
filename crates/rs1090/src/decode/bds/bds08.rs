use deku::prelude::*;
use serde::Serialize;
use std::fmt;
use tracing::{debug, trace};

/**
 * ## Aircraft Identification and Category (BDS 0,8)
 *
 * Designed to broadcast the identification (also known as the "callsign"), and
 * the wake vortex category of the aircraft.
 *
 * | TC  | CA  | C1  | C2  | C3  | C4  | C5  | C6  | C7  | C8  |
 * | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
 * | 5   | 3   | 6   | 6   | 6   | 6   | 6   | 6   | 6   | 6   |
 *
 * TC: Type code CA: Aircraft category C*: A character
 */

#[derive(Debug, PartialEq, DekuRead, Serialize, Clone)]
//#[deku(ctx = "id: u8")]
pub struct AircraftIdentification {
    #[deku(bits = 5)]
    pub id: u8,

    /// The typecode of the aircraft (one of A, B, C, D)
    #[serde(skip)]
    #[deku(skip, default = "Typecode::try_from(*id)?")]
    pub tc: Typecode,

    /// The category of the aircraft
    #[deku(bits = "3")]
    #[serde(skip)]
    pub ca: u8,

    /// Both typecode and category define a wake wortex category.
    #[deku(reader = "wake_vortex(*tc, *ca)")]
    pub wake_vortex: WakeVortex,

    /// Callsign
    #[deku(reader = "callsign_read(deku::reader)")]
    pub callsign: String,
}

#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum Typecode {
    /// Reserved
    D = 1,
    /// Ground vehicles
    C = 2,
    /// Without an engine (glider, hangglider, etc.)
    B = 3,
    /// Aircraft
    #[default]
    A = 4,
}

impl fmt::Display for Typecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::D => "D",
                Self::C => "C",
                Self::B => "B",
                Self::A => "A",
            }
        )
    }
}

use std::convert::TryFrom;

impl TryFrom<u8> for Typecode {
    type Error = DekuError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::D),
            2 => Ok(Self::C),
            3 => Ok(Self::B),
            4 => Ok(Self::A),
            _ => Err(DekuError::InvalidParam(
                "Invalid value for Typecode".into(),
            )),
        }
    }
}

/**
* The CA value in combination with TC value defines the wake vortex category of
* the aircraft.
*
* It is worth noting that ADS-B has its own definition of wake categories,
* which is different from the ICAO wake turbulence category definition commonly
* used in aviation. The relationships of ICAO wake turbulence category (WTC)
* and ADS-B wake vortex category are:
*
* - ICAO WTC L (Light) is equivalent to ADS-B (TC=4, CA=1).
* - ICAO WTC M (Medium) is equivalent to ADS-B (TC=4, CA=2 or CA=3).
* - ICAO WTC H (Heavy) or J (Super) is equivalent to ADS-B (TC=4, CA=5).
*/
#[derive(Debug, PartialEq, Serialize, Copy, Clone)]
pub enum WakeVortex {
    Reserved,

    // Category C
    #[serde(rename = "n/a")]
    NoInformation,
    #[serde(rename = "Surface emergency vehicle")]
    EmergencyVehicle,
    #[serde(rename = "Surface service vehicle")]
    ServiceVehicle,
    Obstruction,

    // Category B
    Glider,
    #[serde(rename = "Lighter than air")]
    Lighter,
    Parachutist,
    Ultralight,
    #[serde(rename = "UAM")]
    Unmanned,
    Space,

    // Category A
    #[serde(rename = "<7000kg")]
    Light,
    #[serde(rename = "<34,000kg")]
    Medium1,
    #[serde(rename = "<136,000kg")]
    Medium2,
    #[serde(rename = "High vortex")]
    HighVortex,
    Heavy,
    #[serde(rename = "High performance")]
    HighPerformance,
    Rotorcraft,
}

impl fmt::Display for WakeVortex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match &self {
            Self::Reserved => "Reserved",
            Self::NoInformation => "No category information",
            Self::EmergencyVehicle => "Surface Emergency Vehicle",
            Self::ServiceVehicle => "Surface Service Vehicle",
            Self::Obstruction => "Ground Obstruction",
            Self::Glider => "Glider, sailplane",
            Self::Lighter => "Lighter than air",
            Self::Parachutist => "Parachutist, Skydiver",
            Self::Ultralight => "Ultralight, hang-glider, paraglider",
            Self::Unmanned => "Unmanned Air Vehicle",
            Self::Space => "Space or transatmospheric vehicle",
            Self::Light => "Light (less than 7000 kg)",
            Self::Medium1 => "Medium 1 (between 7000 kg and 34000 kg)",
            Self::Medium2 => "Medium 2 (between 34000 kg to 136000 kg)",
            Self::HighVortex => "High vortex aircraft",
            Self::Heavy => "Heavy (larger than 136000 kg)",
            Self::HighPerformance => {
                "High performance (>5 g acceleration) and high speed (>400 kt)"
            }
            Self::Rotorcraft => "Rotorcraft",
        };
        write!(f, "{string}")?;
        Ok(())
    }
}

pub fn wake_vortex(tc: Typecode, ca: u8) -> Result<WakeVortex, DekuError> {
    let wake_vortex = match (tc, ca) {
        (Typecode::D, _) => WakeVortex::Reserved,
        (_, 0) => WakeVortex::NoInformation,
        (Typecode::C, 1) => WakeVortex::EmergencyVehicle,
        (Typecode::C, 3) => WakeVortex::ServiceVehicle,
        (Typecode::C, _) => WakeVortex::Obstruction,
        (Typecode::B, 1) => WakeVortex::Glider,
        (Typecode::B, 2) => WakeVortex::Lighter,
        (Typecode::B, 3) => WakeVortex::Parachutist,
        (Typecode::B, 4) => WakeVortex::Ultralight,
        (Typecode::B, 5) => WakeVortex::Reserved,
        (Typecode::B, 6) => WakeVortex::Unmanned,
        (Typecode::B, 7) => WakeVortex::Space,
        (Typecode::A, 1) => WakeVortex::Light,
        (Typecode::A, 2) => WakeVortex::Medium1,
        (Typecode::A, 3) => WakeVortex::Medium2,
        (Typecode::A, 4) => WakeVortex::HighVortex,
        (Typecode::A, 5) => WakeVortex::Heavy,
        (Typecode::A, 6) => WakeVortex::HighPerformance,
        (Typecode::A, 7) => WakeVortex::Rotorcraft,
        _ => WakeVortex::Reserved, // only 3 bits anyway
    };
    Ok(wake_vortex)
}

const CHAR_LOOKUP: &[u8; 64] =
    b"#ABCDEFGHIJKLMNOPQRSTUVWXYZ##### ###############0123456789######";

pub fn callsign_read<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<String, DekuError> {
    let mut chars = vec![];
    for _ in 1..=8 {
        let c = u8::from_reader_with_ctx(reader, deku::ctx::BitSize(6))?;
        trace!("Reading letter {}", CHAR_LOOKUP[c as usize] as char);
        if c != 32 {
            chars.push(c);
        }
    }
    let encoded = chars
        .into_iter()
        .map(|b| CHAR_LOOKUP[b as usize] as char)
        .collect::<String>();

    debug!("Reading callsign {}", encoded);
    Ok(encoded)
}

impl fmt::Display for AircraftIdentification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Aircraft identification and category (BDS 0,8)")?;
        writeln!(f, "  Callsign:      {}", &self.callsign)?;
        writeln!(f, "  Category:      {}", &self.wake_vortex)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_callsign() {
        let bytes = hex!("8d406b902015a678d4d220aa4bda");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            if let ME::BDS08(AircraftIdentification {
                id: _id,
                tc,
                ca,
                callsign,
                wake_vortex,
            }) = adsb_msg.message
            {
                assert_eq!(format!("{tc}{ca}"), "A0");
                assert_eq!(format!("{wake_vortex}"), "No category information");
                assert_eq!(callsign, "EZY85MH");
                return;
            }
        }
        unreachable!();
    }

    #[test]
    fn test_format() {
        let bytes = hex!("8d406b902015a678d4d220aa4bda");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        assert_eq!(
            format!("{msg}"),
            r#" DF17. Extended Squitter
  Address:       406b90
  Air/Ground:    airborne
  Aircraft identification and category (BDS 0,8)
  Callsign:      EZY85MH
  Category:      No category information
"#
        )
    }
}
