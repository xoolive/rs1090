extern crate alloc;

use super::bds::{bds05, bds06, bds08, bds09, bds61, bds62, bds65};
use super::{Capability, ICAO};
use alloc::fmt;
use deku::prelude::*;
use serde::Serialize;

/**
 * An ADS-B frame is 112 bits long and consists of five main parts,
 * shown as follows:
 *
 * +----------+----------+-------------+------------------------+-----------+
 * |  DF (5)  |  CA (3)  |  ICAO (24)  |         ME (56)        |  PI (24)  |
 * +----------+----------+-------------+------------------------+-----------+
 *
 */

#[derive(Debug, PartialEq, DekuRead, Clone, Serialize)]
pub struct ADSB {
    /// Transponder Capability
    #[serde(skip)]
    pub capability: Capability,

    /// ICAO aircraft address
    pub icao24: ICAO,

    /// ME (Typecode)
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

/*
* |  `ME`               |  Name                               |
* | ------------------- | ----------------------------------- |
* | 0                   | [`NoPosition`]                      |
* | 1..=4               | [`AircraftIdentification`]          |
* | 5..=8               | [`SurfacePosition`]                 |
* | 9..=18              | [`AirbornePosition`] (barometric)   |
* | 19                  | [`AirborneVelocity`]                |
* | 20..=22             | [`AirbornePosition`] (GNSS)         |
* | 23                  | [`Reserved0`]                       |
* | 24                  | [`SurfaceSystemStatus`]             |
* | 25..=27             | [`Reserved1`]                       |
* | 28                  | [`AircraftStatus`]                  |
* | 29                  | [`TargetStateAndStatusInformation`] |
* | 30                  | [`AircraftOperationalCoordination`] |
* | 31                  | [`AircraftOperationStatus`]         |
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
    BDS05(bds05::PositionAltitude),

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
    BDS65(bds65::OperationStatus),
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
    use super::*;
    use crate::decode::{Message, DF};
    use hexlit::hex;

    #[test]
    fn test_icao24() {
        let bytes = hex!("8D406B902015A678D4D220AA4BDA");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let DF::ADSB(msg) = msg.df {
            assert_eq!(format!("{}", msg.icao24), "406b90");
            return;
        }
        unreachable!();
    }
}
