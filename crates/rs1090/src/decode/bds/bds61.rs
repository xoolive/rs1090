use crate::decode::IdentityCode;
use deku::prelude::*;
use serde::Serialize;
use std::fmt;

/**
 * ## Aircraft Status (BDS 6,1)
 */
#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct AircraftStatus {
    /// The subtype can be "emergency/priority" or "ACAS RA"
    pub subtype: AircraftStatusType,
    /// The reason for the emergency
    pub emergency_state: EmergencyState,
    /// The 13-bit identity code (squawk)
    pub squawk: IdentityCode,
}

impl fmt::Display for AircraftStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Aircraft Status (BDS 6,1)")?;
        writeln!(f, "  Squawk:        {:x?}", &self.squawk)?;
        writeln!(f, "  Emergency/priority:    {}", &self.emergency_state)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "3")]
#[serde(rename_all = "snake_case")]
pub enum AircraftStatusType {
    #[deku(id = "0")]
    NoInformation,
    #[deku(id = "1")]
    #[serde(rename = "emergency_priority")]
    EmergencyPriorityStatus,
    #[deku(id = "2")]
    #[serde(rename = "acas_ra")]
    ACASRaBroadcast,
    #[deku(id_pat = "_")]
    Reserved,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "3")]
#[serde(rename_all = "snake_case")]
pub enum EmergencyState {
    None = 0,
    General = 1,
    Medical = 2,
    MinimumFuel = 3,
    NoCommunication = 4,
    UnlawfulInterference = 5,
    DownedAircraft = 6,
    Reserved = 7,
}

impl fmt::Display for EmergencyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::None => "No emergency",
            Self::General => "General emergency (7700)",
            Self::Medical => "Lifeguard/Medical emergency",
            Self::MinimumFuel => "Minimum fuel",
            Self::NoCommunication => "No communication (7600)",
            Self::UnlawfulInterference => "Unlawful interference (7500)",
            Self::DownedAircraft => "Downed aircraft",
            Self::Reserved => "Reserved",
        };
        write!(f, "{s}")?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
pub struct OperationCodeSurface {
    #[deku(bits = "1")]
    pub poe: u8,
    #[deku(bits = "1")]
    pub cdti: u8,
    #[deku(bits = "1")]
    pub b2_low: u8,
    #[deku(bits = "3")]
    #[deku(pad_bits_before = "6")]
    pub lw: u8,
}
