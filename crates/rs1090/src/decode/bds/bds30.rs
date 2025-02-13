use deku::prelude::*;
use serde::Serialize;

use crate::decode::{AC13Field, ICAO};

/**
 * ## ACAS active resolution advisory (BDS 3,0)
 *
 * The BDS 3,0 message is used to report resolution advisories (RA) generated by
 * ACAS equipment.
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[serde(tag = "bds", rename = "30")]
pub struct ACASResolutionAdvisory {
    #[deku(bits = "8", map = "fail_if_not30")]
    #[serde(skip)]
    /// The first eight bits indicate the BDS code 0011 0000 (3,0 in hexadecimal).
    pub bds: u8,

    #[deku(bits = "1")]
    /// Active resolution advisories.
    /// False if no RA or multiple thread/different directions.
    pub issued_ra: bool,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Active resolution advisories: corrective/preventive
    pub corrective: Option<bool>,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Active resolution advisories: downward/upward
    pub downward_sense: Option<bool>,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Active resolution advisories:
    pub increased_rate: Option<bool>,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Active resolution advisories:
    pub sense_reversal: Option<bool>,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Active resolution advisories:
    pub altitude_crossing: Option<bool>,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Active resolution advisories: positive/vertical speed limit
    pub positive: Option<bool>,

    #[deku(bits = "7")]
    #[serde(skip)]
    /// Active resolution advisories: reserved for ACAS III
    pub reserved_acas3: u16,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolution advisory complements record: do not pass below
    pub no_below: Option<bool>,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolution advisory complements record: do not pass above
    pub no_above: Option<bool>,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolution advisory complements record: do not turn left
    pub no_left: Option<bool>,

    #[deku(
        bits = "1",
        map = "|v: bool| -> Result<_, DekuError> {
            if *issued_ra { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolution advisory complements record: do not turn right
    pub no_right: Option<bool>,

    #[deku(bits = "1")]
    /// RA terminated
    pub terminated: bool,

    #[deku(bits = "1")]
    /// Multiple threat encounter (not supported)
    pub multiple: bool,

    /// Threat type indicator
    #[serde(flatten)]
    pub threat_type: ThreatType,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[deku(id_type = "u8", bits = "2")]
#[serde(untagged)]
pub enum ThreatType {
    #[deku(id = "0")]
    NoIdentity {
        #[deku(bits = "26")]
        #[serde(skip)]
        unused: u32,
    },

    #[deku(id = "1")]
    ThreatAddress(ThreadAddress),

    #[deku(id = "2")]
    ThreatOrientation(ThreatOrientation),

    #[deku(id = "3")]
    NotAssigned {
        #[deku(bits = "26")]
        #[serde(skip)]
        unused: u32,
    },
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct ThreadAddress {
    /// Threat identity data (icao24).
    pub threat_identity: ICAO,

    #[deku(bits = "2")]
    #[serde(skip)]
    pub zeros: u8,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct ThreatOrientation {
    /// Altitude code on 13 bits
    #[serde(rename = "threat_altitude")]
    altitude: AC13Field,

    #[deku(
        bits = "7",
        map = "|n: u8| -> Result<_, DekuError> {
            if n == 0 { Ok(None) } else { Ok(Some((n as f32 - 1.) / 10.)) }
        }"
    )]
    /// Most recent threat range from ACAS (max 12.55 nautical miles)
    #[serde(rename = "threat_range")]
    range: Option<f32>,

    #[deku(
        bits = "6",
        map = "|n: u16| -> Result<_, DekuError> {
            if n == 0 { Ok(None) } else { Ok(Some(6 * (n - 1) + 3)) }
        }"
    )]
    /// Most recent estimated bearing of the threat aircraft,
    /// relative to their own heading (3 degree precision)
    #[serde(rename = "threat_bearing")]
    bearing: Option<u16>,
}

fn fail_if_not30(value: u8) -> Result<u8, DekuError> {
    if value == 0x30 {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "First bits must be 0x30 in BDS 3,0".into(),
        ))
    }
}
