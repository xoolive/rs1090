extern crate alloc;

use alloc::fmt;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * +------+------+------+------+------+------+------+------+------+------+
 * | TC,5 | CA,3 | C1,6 | C2,6 | C3,6 | C4,6 | C5,6 | C6,6 | C7,6 | C8,6 |
 * +------+------+------+------+------+------+------+------+------+------+
 *
 * TC: Type code
 * CA: Aircraft category
 * C*: A character
 */

#[derive(Debug, PartialEq, DekuRead, Serialize, Clone)]
pub struct AircraftIdentification {
    #[serde(skip)]
    pub tc: TypeCoding,

    #[deku(bits = "3")]
    #[serde(skip)]
    pub ca: u8,

    #[deku(reader = "wake_vortex(deku::rest, *tc, *ca)")]
    pub wake_vortex: WakeVortex,

    /// Callsign
    #[deku(reader = "callsign_read(deku::rest)")]
    pub callsign: String,
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "5")]
pub enum TypeCoding {
    D = 1,
    C = 2,
    B = 3,
    A = 4,
}

impl fmt::Display for TypeCoding {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
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

pub fn wake_vortex(
    rest: &BitSlice<u8, Msb0>,
    tc: TypeCoding,
    ca: u8,
) -> Result<(&BitSlice<u8, Msb0>, WakeVortex), DekuError> {
    let wake_vortex = match (tc, ca) {
        (TypeCoding::D, _) => WakeVortex::Reserved,
        (_, 0) => WakeVortex::NoInformation,
        (TypeCoding::C, 1) => WakeVortex::EmergencyVehicle,
        (TypeCoding::C, 3) => WakeVortex::ServiceVehicle,
        (TypeCoding::C, _) => WakeVortex::Obstruction,
        (TypeCoding::B, 1) => WakeVortex::Glider,
        (TypeCoding::B, 2) => WakeVortex::Lighter,
        (TypeCoding::B, 3) => WakeVortex::Parachutist,
        (TypeCoding::B, 4) => WakeVortex::Ultralight,
        (TypeCoding::B, 5) => WakeVortex::Reserved,
        (TypeCoding::B, 6) => WakeVortex::Unmanned,
        (TypeCoding::B, 7) => WakeVortex::Space,
        (TypeCoding::A, 1) => WakeVortex::Light,
        (TypeCoding::A, 2) => WakeVortex::Medium1,
        (TypeCoding::A, 3) => WakeVortex::Medium2,
        (TypeCoding::A, 4) => WakeVortex::HighVortex,
        (TypeCoding::A, 5) => WakeVortex::Heavy,
        (TypeCoding::A, 6) => WakeVortex::HighPerformance,
        (TypeCoding::A, 7) => WakeVortex::Rotorcraft,
        _ => WakeVortex::Reserved, // only 3 bits anyway
    };
    Ok((rest, wake_vortex))
}

const CHAR_LOOKUP: &[u8; 64] =
    b"#ABCDEFGHIJKLMNOPQRSTUVWXYZ##### ###############0123456789######";

pub fn callsign_read(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, String), DekuError> {
    let mut inside_rest = rest;

    let mut chars = vec![];
    for _ in 0..=6 {
        let (for_rest, c) = <u8>::read(inside_rest, deku::ctx::BitSize(6))?;
        if c != 32 {
            chars.push(c);
        }
        inside_rest = for_rest;
    }
    let encoded = chars
        .into_iter()
        .map(|b| CHAR_LOOKUP[b as usize] as char)
        .collect::<String>();

    Ok((inside_rest, encoded))
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
    use super::*;
    use crate::decode::ME::BDS08;
    use crate::decode::{Message, DF::ADSB};
    use hexlit::hex;

    #[test]
    fn test_callsign() {
        let bytes = hex!("8d406b902015a678d4d220aa4bda");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ADSB(adsb_msg) = msg.df {
            if let BDS08(AircraftIdentification {
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
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
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
