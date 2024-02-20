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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ADSB {
    /// Transponder Capability
    #[serde(skip_serializing)]
    pub capability: Capability,
    /// ICAO aircraft address
    pub icao24: ICAO,
    /// Typecode
    #[serde(skip_serializing)]
    pub message: Typecode, // We only read the typecode here, then distribute
    /// Parity/Interrogator ID
    #[serde(skip_serializing)]
    pub parity: ICAO,
}

impl ADSB {
    pub(crate) fn to_string(&self) -> Result<String, fmt::Error> {
        let mut f = String::new();
        write!(
            f,
            "{}",
            self.message
                .to_string(self.icao24, "(Mode S / ADS-B)", self.capability, true)?
        )?;
        Ok(f)
    }
}

/*
* |  `Typecode`         |  Name                              |
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

// TODO also implement uncertainty here

#[derive(Debug, PartialEq, DekuRead, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[deku(type = "u8", bits = "5")]
pub enum Typecode {
    #[deku(id = "0")]
    NoPosition([u8; 6]),

    #[deku(id_pat = "1..=4")]
    AircraftIdentification(bds08::Identification),

    #[deku(id_pat = "5..=8")]
    SurfacePosition(bds06::SurfacePosition),

    #[deku(id_pat = "9..=18")]
    AirbornePositionBaroAltitude(bds05::PositionAltitude),

    #[deku(id = "19")]
    AirborneVelocity(bds09::AirborneVelocity),

    #[deku(id_pat = "20..=22")]
    AirbornePositionGNSSAltitude(bds05::PositionAltitude),

    #[deku(id = "23")]
    Reserved0([u8; 6]),

    #[deku(id_pat = "24")]
    SurfaceSystemStatus([u8; 6]),

    #[deku(id_pat = "25..=27")]
    Reserved1([u8; 6]),

    #[deku(id = "28")]
    AircraftStatus(bds61::AircraftStatus),

    #[deku(id = "29")]
    TargetStateAndStatusInformation(bds62::TargetStateAndStatusInformation),

    #[deku(id = "30")]
    AircraftOperationalCoordination([u8; 6]),

    #[deku(id = "31")]
    AircraftOperationStatus(bds65::OperationStatus),
}

impl Typecode {
    pub(crate) fn to_string(
        &self,
        icao: ICAO,
        address_type: &str,
        capability: Capability,
        is_transponder: bool,
    ) -> Result<String, fmt::Error> {
        let transponder = match is_transponder {
            true => " ",
            false => " (Non-Transponder) ",
        };
        let mut f = String::new();
        match self {
            Typecode::NoPosition(_) => {
                writeln!(f, " Extended Squitter{transponder}No position information")?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
            }
            Typecode::AircraftIdentification(bds08::Identification { tc, ca, callsign }) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Aircraft identification and category"
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                writeln!(f, "  Callsign:      {callsign}")?;
                writeln!(f, "  Category:      {tc}{ca}")?;
            }
            Typecode::SurfacePosition(surface_position) => {
                writeln!(f, " Extended Squitter{transponder}Surface position")?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                write!(f, "{surface_position}")?;
            }
            Typecode::AirbornePositionBaroAltitude(position_baro) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Airborne position (barometric altitude)"
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(f, "{position_baro}")?;
            }
            Typecode::AirborneVelocity(airborne_velocity) => match &airborne_velocity.sub_type {
                bds09::AirborneVelocitySubType::GroundSpeedDecoding(_) => {
                    writeln!(
                        f,
                        " Extended Squitter{transponder}Airborne velocity over ground, subsonic"
                    )?;
                    writeln!(f, "  Address:       {icao} {address_type}")?;
                    writeln!(f, "  Air/Ground:    {capability}")?;
                    writeln!(
                        f,
                        "  GNSS delta:    {}{} ft",
                        airborne_velocity.gnss_sign, airborne_velocity.gnss_baro_diff
                    )?;
                    if let Some((track, ground_speed, vertical_rate)) =
                        airborne_velocity.calculate()
                    {
                        writeln!(f, "  Track angle:   {}", libm::ceil(track as f64))?;
                        writeln!(
                            f,
                            "  Speed:         {} kt groundspeed",
                            libm::floor(ground_speed)
                        )?;
                        writeln!(
                            f,
                            "  Vertical rate: {} ft/min {}",
                            vertical_rate, airborne_velocity.vrate_src
                        )?;
                    } else {
                        writeln!(f, "  Invalid packet")?;
                    }
                }
                bds09::AirborneVelocitySubType::AirspeedDecoding(airspeed_decoding) => {
                    writeln!(
                        f,
                        " Extended Squitter{transponder}Airspeed and track, subsonic",
                    )?;
                    writeln!(f, "  Address:       {icao} {address_type}")?;
                    writeln!(f, "  Air/Ground:    {capability}")?;
                    writeln!(
                        f,
                        "  {}:           {} kt",
                        airspeed_decoding.airspeed_type, airspeed_decoding.airspeed
                    )?;
                    if airborne_velocity.vrate_value > 0 {
                        writeln!(
                            f,
                            "  Baro rate:     {}{} ft/min",
                            airborne_velocity.vrate_sign,
                            (airborne_velocity.vrate_value - 1) * 64
                        )?;
                    }
                    writeln!(f, "  NACv:          {}", airborne_velocity.nac_v)?;
                }
                bds09::AirborneVelocitySubType::Reserved0(_)
                | bds09::AirborneVelocitySubType::Reserved1(_) => {
                    writeln!(
                        f,
                        " Extended Squitter{transponder}Airborne Velocity status (reserved)",
                    )?;
                    writeln!(f, "  Address:       {icao} {address_type}")?;
                }
            },
            Typecode::AirbornePositionGNSSAltitude(position_gnss) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Airborne position (GNSS altitude)",
                )?;
                writeln!(f, "  Address:      {icao} {address_type}")?;
                write!(f, "{position_gnss}")?;
            }
            Typecode::Reserved0(_) | Typecode::Reserved1(_) => {
                writeln!(f, " Extended Squitter{transponder}Unknown")?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
            }
            Typecode::SurfaceSystemStatus(_) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Reserved for surface system status",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
            }
            Typecode::AircraftStatus(bds61::AircraftStatus {
                emergency_state,
                squawk,
                ..
            }) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Emergency/priority status",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                writeln!(f, "  Squawk:        {squawk:x?}")?;
                writeln!(f, "  Emergency/priority:    {emergency_state}")?;
            }
            Typecode::TargetStateAndStatusInformation(target_info) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Target state and status (V2)",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                writeln!(f, "  Target State and Status:")?;
                writeln!(f, "    Target altitude:   MCP, {} ft", target_info.altitude)?;
                writeln!(f, "    Altimeter setting: {} millibars", target_info.qnh)?;
                if target_info.is_heading {
                    writeln!(f, "    Target heading:    {}", target_info.heading)?;
                }
                if target_info.tcas {
                    write!(f, "    ACAS:              operational ")?;
                    if target_info.autopilot {
                        write!(f, "autopilot ")?;
                    }
                    if target_info.vnac {
                        write!(f, "vnav ")?;
                    }
                    if target_info.alt_hold {
                        write!(f, "altitude-hold ")?;
                    }
                    if target_info.approach {
                        write!(f, " approach")?;
                    }
                    writeln!(f)?;
                } else {
                    writeln!(f, "    ACAS:              NOT operational")?;
                }
                writeln!(f, "    NACp:              {}", target_info.nacp)?;
                writeln!(f, "    NICbaro:           {}", target_info.nicbaro)?;
                writeln!(f, "    SIL:               {} (per sample)", target_info.sil)?;
                writeln!(f, "    QNH:               {} millibars", target_info.qnh)?;
            }
            Typecode::AircraftOperationalCoordination(_) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Aircraft Operational Coordination",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
            }
            Typecode::AircraftOperationStatus(bds65::OperationStatus::Airborne(
                opstatus_airborne,
            )) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Aircraft operational status (airborne)",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(f, "  Aircraft Operational Status:\n{opstatus_airborne}")?;
            }
            Typecode::AircraftOperationStatus(bds65::OperationStatus::Surface(
                opstatus_surface,
            )) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Aircraft operational status (surface)",
                )?;
                writeln!(f, "  Address:       {icao} {address_type}")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
                write!(f, "  Aircraft Operational Status:\n {opstatus_surface}")?;
            }
            Typecode::AircraftOperationStatus(bds65::OperationStatus::Reserved(..)) => {
                writeln!(
                    f,
                    " Extended Squitter{transponder}Aircraft operational status (reserved)",
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
