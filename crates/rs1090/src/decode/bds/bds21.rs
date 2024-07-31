use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use regex::Regex;
use serde::Serialize;

/**
 * ## Aircraft and airline registration markings (BDS 2,1)
 *
 * To permit ground systems to identify the aircraft without the
 * necessity of compiling and maintaining continuously updated data banks.
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[serde(tag = "bds", rename = "21")]
pub struct AircraftAndAirlineRegistrationMarkings {
    #[deku(bits = "1")]
    #[serde(skip)]
    pub ac_status: bool,

    #[deku(reader = "aircraft_registration_read(deku::rest, *ac_status)")]
    #[serde(rename = "registration")]
    pub aircraft_registration: Option<String>,

    #[deku(bits = "1")]
    #[serde(skip)]
    pub al_status: bool,

    #[deku(reader = "airline_registration_read(deku::rest, *al_status)")]
    #[serde(rename = "airline", skip_serializing_if = "Option::is_none")]
    pub airline_registration: Option<String>,
}

const CHAR_LOOKUP: &[u8; 64] =
    b"#ABCDEFGHIJKLMNOPQRSTUVWXYZ##### ###############0123456789######";

pub fn aircraft_registration_read(
    rest: &BitSlice<u8, Msb0>,
    status: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<String>), DekuError> {
    let mut inside_rest = rest;

    let mut chars = vec![];
    for _ in 0..=6 {
        let (for_rest, c) = <u8>::read(inside_rest, deku::ctx::BitSize(6))?;
        if c != 32 {
            chars.push(c);
        }
        inside_rest = for_rest;
    }

    let all_zeros = chars.iter().all(|&x| x == 0);
    let encoded = chars
        .into_iter()
        .map(|b| CHAR_LOOKUP[b as usize] as char)
        .collect::<String>();

    if status {
        let re = Regex::new(r"^[A-Z0-9]+[\s#]?[A-Z0-9]+$").unwrap();
        if re.is_match(&encoded) {
            Ok((inside_rest, Some(encoded)))
        } else {
            Err(DekuError::Assertion(format!(
                "Invalid aircraft registration {}",
                encoded
            )))
        }
    } else if all_zeros {
        Ok((inside_rest, None))
    } else {
        Err(DekuError::Assertion(format!(
            "Non-null value after invalid aircraft registration status: {}",
            encoded
        )))
    }
}

pub fn airline_registration_read(
    rest: &BitSlice<u8, Msb0>,
    status: bool,
) -> Result<(&BitSlice<u8, Msb0>, Option<String>), DekuError> {
    let mut inside_rest = rest;

    let mut chars = vec![];
    for _ in 0..=2 {
        let (for_rest, c) = <u8>::read(inside_rest, deku::ctx::BitSize(6))?;
        if c != 32 {
            chars.push(c);
        }
        inside_rest = for_rest;
    }
    let all_zeros = chars.iter().all(|&x| x == 0);
    let encoded = chars
        .into_iter()
        .map(|b| CHAR_LOOKUP[b as usize] as char)
        .collect::<String>();

    if status {
        // Ok((inside_rest, Some(encoded)))
        Err(DekuError::Assertion(format!(
            "Most transponders don't implement this field. (value = {})",
            encoded
        )))
    } else if all_zeros {
        Ok((inside_rest, None))
    } else {
        Err(DekuError::Assertion(format!(
            "Non-null value after invalid airline registration status: {}",
            encoded
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_valid_bds21() {
        let bytes = hex!("a00002bf940f19680c0000000000");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            let AircraftAndAirlineRegistrationMarkings {
                aircraft_registration,
                ..
            } = bds.bds21.unwrap();
            assert_eq!(aircraft_registration, Some("JA824A".to_string()));
        } else {
            unreachable!();
        }

        let bytes = hex!("a00002988230c3b470a000000000");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            let AircraftAndAirlineRegistrationMarkings {
                aircraft_registration,
                ..
            } = bds.bds21.unwrap();
            assert_eq!(aircraft_registration, Some("AFFGZNE".to_string()));
        } else {
            unreachable!();
        }
        let bytes = hex!("a0000793ac45ab164c0000000000");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            let AircraftAndAirlineRegistrationMarkings {
                aircraft_registration,
                ..
            } = bds.bds21.unwrap();
            assert_eq!(aircraft_registration, Some("VH#VKI".to_string()));
        } else {
            unreachable!();
        }
    }
}
