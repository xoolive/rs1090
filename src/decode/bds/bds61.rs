extern crate alloc;

use crate::decode::decode_id13;
use alloc::fmt;
use deku::prelude::*;
use serde::ser::{Serialize, SerializeStruct, Serializer};

/// Table: A-2-97
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
pub struct AircraftStatus {
    pub sub_type: AircraftStatusType,
    pub emergency_state: EmergencyState,
    #[deku(
        bits = "13",
        endian = "big",
        map = "|squawk: u32| -> Result<_, DekuError> {Ok(decode_id13(squawk))}"
    )]
    pub squawk: u32,
}

impl Serialize for AircraftStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Message", 2)?;
        state.serialize_field("squawk", &self.squawk)?;
        let emergency = format!("{}", &self.emergency_state);
        state.serialize_field("emergency", &emergency)?;
        state.end()
    }
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
pub enum AircraftStatusType {
    #[deku(id = "0")]
    NoInformation,
    #[deku(id = "1")]
    EmergencyPriorityStatus,
    #[deku(id = "2")]
    ACASRaBroadcast,
    #[deku(id_pat = "_")]
    Reserved,
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
pub enum EmergencyState {
    None = 0,
    General = 1,
    Lifeguard = 2,
    MinimumFuel = 3,
    NoCommunication = 4,
    UnlawfulCommunication = 5,
    Reserved = 6,
    Reserved2 = 7,
}

impl fmt::Display for EmergencyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::None => "No emergency",
            Self::General => "General emergency",
            Self::Lifeguard => "Lifeguard/medical",
            Self::MinimumFuel => "Minimum fuel",
            Self::NoCommunication => "No communication",
            Self::UnlawfulCommunication => "Unlawful communications",
            Self::Reserved => "reserved",
            Self::Reserved2 => "reserved",
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
