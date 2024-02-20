extern crate alloc;

use alloc::fmt;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;

/**
 * +------+------+------+------+------+------+------+------+------+------+
 * | TC,5 | CA,3 | C1,6 | C2,6 | C3,6 | C4,6 | C5,6 | C6,6 | C7,6 | C8,6 |
 * +------+------+------+------+------+------+------+------+------+------+
 *
 * TC: Type code
 * CA: Aircraft category
 * C*: A character
 */

#[derive(Debug, PartialEq, Eq, DekuRead, Clone)]
pub struct Identification {
    pub tc: TypeCoding,

    #[deku(bits = "3")]
    pub ca: u8,

    /// Callsign
    #[deku(reader = "callsign_read(deku::rest)")]
    pub callsign: String,
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
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

/* TODO Wake Vortex category
* TC 	CA 	Category
* 1 	ANY 	Reserved
* ANY 	0 	No category information
* 2 	1 	Surface emergency vehicle
* 2 	3 	Surface service vehicle
* 2 	4–7 	Ground obstruction
* 3 	1 	Glider, sailplane
* 3 	2 	Lighter-than-air
* 3 	3 	Parachutist, skydiver
* 3 	4 	Ultralight, hang-glider, paraglider
* 3 	5 	Reserved
* 3 	6 	Unmanned aerial vehicle
* 3 	7 	Space or transatmospheric vehicle
* 4 	1 	Light (less than 7000 kg)
* 4 	2 	Medium 1 (between 7000 kg and 34000 kg)
* 4 	3 	Medium 2 (between 34000 kg to 136000 kg)
* 4 	4 	High vortex aircraft
* 4 	5 	Heavy (larger than 136000 kg)
* 4 	6 	High performance (>5 g acceleration) and high speed (>400 kt)
* 4 	7 	Rotorcraft
*/

const CHAR_LOOKUP: &[u8; 64] = b"#ABCDEFGHIJKLMNOPQRSTUVWXYZ##### ###############0123456789######";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Typecode::AircraftIdentification;
    use crate::decode::{Message, DF::ADSB};
    use hexlit::hex;

    #[test]
    fn test_callsign() {
        let bytes = hex!("8d406b902015a678d4d220aa4bda");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ADSB(adsb_msg) = msg.df {
            if let AircraftIdentification(Identification { tc, ca, callsign }) = adsb_msg.message {
                assert_eq!(format!("{tc}"), "A");
                assert_eq!(ca, 0);
                assert_eq!(callsign, "EZY85MH");
                return;
            }
        }
        unreachable!();
    }
}
