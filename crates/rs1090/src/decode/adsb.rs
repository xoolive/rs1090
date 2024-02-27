use super::bds::{bds05, bds06, bds08, bds09, bds61, bds62, bds65};
use super::{Capability, ICAO};
use deku::prelude::*;
use serde::Serialize;
use std::fmt;

/**
 * An ADS-B frame is 112 bits long.
 *
 * It consists of five main parts, shown as follows:
 *
 * | DF  | CA  | ICAO | ME  | PI  |
 * | --- | --- | ---- | --- | --- |
 * | 5   | 3   | 24   | 56  | 24  |
 *
 */

#[derive(Debug, PartialEq, DekuRead, Clone, Serialize)]
pub struct ADSB {
    /// The transponder capability
    #[serde(skip)]
    pub capability: Capability,

    /// The ICAO aircraft address on 24 bytes
    pub icao24: ICAO,

    /// The message, prefixed by a typecode on 5 bytes
    #[serde(flatten)]
    pub message: ME,

    /// Parity/Interrogator ID
    #[serde(skip)]
    pub parity: ICAO,
}

impl fmt::Display for ADSB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, " DF17. Extended Squitter")?;
        writeln!(f, "  Address:       {}", &self.icao24)?;
        writeln!(f, "  Air/Ground:    {}", &self.capability)?;
        write!(f, "{}", &self.message)
    }
}

/**
* The first 5 bytes of the Message Field [`ME`] encode the typecode.
*
* It is used to identify which kind of data is encode in the following bytes.
*
* | Typecode | Name                                              |
* | -------- | ------------------------------------------------- |
* | 0        | [`ME::NoPosition`]                                |
* | 1..=4    | [`bds08::AircraftIdentification`]                 |
* | 5..=8    | [`bds06::SurfacePosition`]                        |
* | 9..=18   | [`bds05::AirbornePosition`] (barometric altitude) |
* | 19       | [`bds09::AirborneVelocity`]                       |
* | 20..=22  | [`bds05::AirbornePosition`] (GNSS height)         |
* | 23       | [`ME::Reserved0`]                                 |
* | 24       | [`ME::SurfaceSystemStatus`]                       |
* | 25..=27  | [`ME::Reserved1`]                                 |
* | 28       | [`bds61::AircraftStatus`]                         |
* | 29       | [`bds62::TargetStateAndStatusInformation`]        |
* | 30       | [`ME::AircraftOperationalCoordination`]           |
* | 31       | [`bds65::AircraftOperationStatus`]                |
*/

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[deku(type = "u8", bits = "5")]
//#[serde(untagged)]
#[serde(tag = "BDS")]
pub enum ME {
    #[deku(id = "0")]
    #[serde(skip)]
    NoPosition([u8; 6]),

    #[deku(id_pat = "1..=4")]
    #[serde(rename = "0,8")]
    BDS08(bds08::AircraftIdentification),

    #[deku(id_pat = "5..=8")]
    #[serde(rename = "0,6")]
    BDS06(bds06::SurfacePosition),

    #[deku(id_pat = "9..=18 | 20..=22")]
    #[serde(rename = "0,5")]
    BDS05(bds05::AirbornePosition),

    #[deku(id = "19")]
    #[serde(rename = "0,9")]
    BDS09(bds09::AirborneVelocity),

    #[deku(id = "23")]
    #[serde(skip)]
    Reserved0([u8; 6]),

    #[deku(id_pat = "24")]
    #[serde(skip)]
    SurfaceSystemStatus([u8; 6]),

    #[deku(id_pat = "25..=27")]
    #[serde(skip)]
    Reserved1([u8; 6]),

    #[deku(id = "28")]
    #[serde(rename = "6,1")]
    BDS61(bds61::AircraftStatus),

    #[deku(id = "29")]
    #[serde(rename = "6,2")]
    BDS62(bds62::TargetStateAndStatusInformation),

    #[deku(id = "30")]
    #[serde(skip)]
    AircraftOperationalCoordination([u8; 6]),

    #[deku(id = "31")]
    #[serde(rename = "6,5")]
    BDS65(bds65::AircraftOperationStatus),
}

impl fmt::Display for ME {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ME::NoPosition(_)
            | ME::Reserved0(_)
            | ME::Reserved1(_)
            | ME::SurfaceSystemStatus(_)
            | ME::AircraftOperationalCoordination(_) => Ok(()),
            ME::BDS05(me) => {
                write!(f, "{}", me)
            }
            ME::BDS06(me) => {
                write!(f, "{}", me)
            }
            ME::BDS08(me) => {
                write!(f, "{}", me)
            }
            ME::BDS09(me) => {
                write!(f, "{}", me)
            }
            ME::BDS61(me) => {
                write!(f, "{}", me)
            }
            ME::BDS62(me) => {
                write!(f, "{}", me)
            }
            ME::BDS65(me) => {
                write!(f, "{}", me)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_icao24() {
        let bytes = hex!("8D406B902015A678D4D220AA4BDA");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ExtendedSquitterADSB(msg) = msg.df {
            assert_eq!(format!("{}", msg.icao24), "406b90");
            return;
        }
        unreachable!();
    }
}
