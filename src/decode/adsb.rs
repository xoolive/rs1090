extern crate alloc;

use super::bds::{bds05, bds06, bds08, bds09, bds61, bds62, bds65};
use super::{Capability, ICAO};
use alloc::fmt;
use deku::prelude::*;
use serde::Serialize;
use std::fmt::Write;

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

impl ADSB {
    pub(crate) fn to_string(&self) -> Result<String, fmt::Error> {
        let mut f = String::new();
        write!(
            f,
            "{}",
            self.message.to_string(
                self.icao24,
                "(Mode S / ADS-B)",
                self.capability
            )?
        )?;
        Ok(f)
    }
}

/*
* |  `ME`               |  Name                              |
* | ------------------- | ---------------------------------- |
* | 0                   | [`NoPosition`]                     |
* | 1..=4               | [`AircraftIdentification`]         |
* | 5..=8               | [`SurfacePosition`]                |
* | 9..=18              | [`AirbornePositionBaroAltitude`]   |
* | 19                  | [`AirborneVelocity`]               |
* | 20..=22             | [`AirbornePositionGNSSAltitude`]   |
* | 23                  | [`Reserved0`]                      |
* | 24                  | [`SurfaceSystemStatus`]            |
* | 25..=27             | [`Reserved1`]                      |
* | 28                  | [`AircraftStatus`]                 |
* | 29                  | [`TargetStateAndStatusInformation`]|
* | 30                  | [`AircraftOperationalCoordination`]|
* | 31                  | [`AircraftOperationStatus`]        |
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

impl ME {
    pub(crate) fn to_string(
        &self,
        icao: ICAO,
        address_type: &str,
        capability: Capability,
    ) -> Result<String, fmt::Error> {
        let mut f = String::new();
        match self {
            ME::NoPosition(_) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter No position information"
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
            }
            ME::BDS05(airborne_position) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Airborne position (BDS 0,5)"
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(f, "{airborne_position}")?;
            }
            ME::BDS06(surface_position) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Surface position (BDS 0,6)"
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(f, "{surface_position}")?;
            }
            ME::BDS08(aircraft_identification) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Aircraft identification and category (BDS 0,8)"
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(f, "{aircraft_identification}")?;
            }
            ME::BDS09(airborne_velocity) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Airborne velocity over ground (BDS 0,9)"
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(f, "{airborne_velocity}")?;
            }
            ME::Reserved0(_) | ME::Reserved1(_) => {
                writeln!(f, " DF17. Extended Squitter Unknown")?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
            }
            ME::SurfaceSystemStatus(_) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Reserved for surface system status",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
            }
            ME::BDS61(bds61::AircraftStatus {
                emergency_state,
                squawk,
                ..
            }) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Emergency/priority status",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                writeln!(f, "  Squawk:        {squawk:x?}")?;
                writeln!(f, "  Emergency/priority:    {emergency_state}")?;
            }
            ME::BDS62(target_info) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Target state and status (BDS 6,2)",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(f, "{target_info}")?;
            }
            ME::AircraftOperationalCoordination(_) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Aircraft Operational Coordination",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
            }
            ME::BDS65(bds65::OperationStatus::Airborne(opstatus_airborne)) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Aircraft operational status (airborne)",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(
                    f,
                    "  Aircraft Operational Status:\n{opstatus_airborne}"
                )?;
            }
            ME::BDS65(bds65::OperationStatus::Surface(opstatus_surface)) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Aircraft operational status (surface)",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(
                    f,
                    "  Aircraft Operational Status:\n {opstatus_surface}"
                )?;
            }
            ME::BDS65(bds65::OperationStatus::Reserved(..)) => {
                writeln!(
                    f,
                    " DF17. Extended Squitter Aircraft operational status (reserved)",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
            }
        }
        Ok(f)
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
