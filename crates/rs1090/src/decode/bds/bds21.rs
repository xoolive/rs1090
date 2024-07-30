use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
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
    pub aircraft_registration: Option<String>,

    #[deku(bits = "1")]
    #[serde(skip)]
    pub al_status: bool,

    #[deku(reader = "airline_registration_read(deku::rest, *al_status)")]
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
    for _ in 0..=7 {
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
        if encoded.starts_with('#') {
            return Err(DekuError::Assertion(
                "Valid aircraft registration starting with an invalid character"
                    .to_string(),
            ));
        }
        Ok((inside_rest, Some(encoded)))
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
        Ok((inside_rest, Some(encoded)))
    } else if all_zeros {
        Ok((inside_rest, None))
    } else {
        Err(DekuError::Assertion(format!(
            "Non-null value after invalid airline registration status: {}",
            encoded
        )))
    }
}
