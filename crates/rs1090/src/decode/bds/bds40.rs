#![allow(clippy::suspicious_else_formatting)]

use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Selected vertical intention (BDS 4,0)
 *
 * The selected vertical intention message is designed for air traffic control
 * to obtain an aircraft’s current vertical intentions. For example, an aircraft
 * controller can use this information to check whether an aircraft is complying
 * with an altitude command.
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct SelectedVerticalIntention {
    #[deku(reader = "read_selected(deku::rest)")]
    #[serde(rename = "selected_mcp", skip_serializing_if = "Option::is_none")]
    pub selected_altitude_mcp: Option<u16>, // 1+12

    #[deku(reader = "read_selected(deku::rest)")]
    #[serde(rename = "selected_fms", skip_serializing_if = "Option::is_none")]
    pub selected_altitude_fms: Option<u16>, //1+12

    #[deku(reader = "read_qnh(deku::rest)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barometric_setting: Option<f64>, //1+12

    #[serde(skip)]
    #[deku(map = "|v: u8| {
        if v == 0 { Ok(v) } else {
            Err(DekuError::Assertion(\"Reserved bits to 0\".to_string()))
        }
    }")]
    #[deku(bits = 8)]
    pub reserved: u8, // 8 bits all zeros

    /// Status of MCP/FCU mode (usually just false)
    #[deku(bits = 1)]
    #[serde(skip)]
    pub mcp_status: bool,
    #[deku(bits = 1)]
    #[serde(skip)]
    pub vnav_mode: bool,
    #[deku(bits = 1)]
    #[serde(skip)]
    pub alt_hold_mode: bool,
    #[deku(bits = 1)]
    #[serde(skip)]
    pub approach_mode: bool,

    #[deku(map = "|v: u8| {
        if v == 0 { Ok(v) } else {
            Err(DekuError::Assertion(\"Reserved bits to 0\".to_string()))
        }
    }")]
    #[deku(bits = 2)]
    #[serde(skip)]
    pub reserved1: u8, // 2 bits all zeros

    #[deku(bits = 1)]
    #[serde(skip)]
    /// Status of target altitude source
    pub source_status: bool,
    #[serde(
        rename = "target_source",
        skip_serializing_if = "TargetSource::is_unknown"
    )]
    /// Target altitude source
    pub target_altitude_source: TargetSource,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[deku(type = "u8", bits = "2")]
pub enum TargetSource {
    #[deku(id = "0")]
    Unknown,
    #[deku(id = "1")]
    AircraftAltitude,
    #[deku(id = "2")]
    FcpMcuSelectedAltitude,
    #[deku(id = "3")]
    FmsSelectedAltitude,
}
impl TargetSource {
    fn is_unknown(&self) -> bool {
        self == &Self::Unknown
    }
}

fn read_selected(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<u16>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(12)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 4,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }
    let value = value * 16;
    // (encoded as a multiple of 16, but rounded to the closest 100 ft)
    let value = (value + 8) / 100 * 100;
    if value > 45000 {
        return Err(DekuError::Assertion("BDS 4,0 status".to_string()));
    }

    Ok((rest, Some(value)))
}

fn read_qnh(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<f64>), DekuError> {
    let (rest, status) =
        bool::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(1)))?;
    let (rest, value) =
        u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(12)))?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion("BDS 4,0 status".to_string()));
        } else {
            return Ok((rest, None));
        }
    }

    Ok((rest, Some(value as f64 * 0.1 + 800.)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_valid_bds40() {
        let bytes = hex!("a000029c85e42f313000007047d3");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            let SelectedVerticalIntention {
                selected_altitude_fms,
                selected_altitude_mcp,
                barometric_setting,
                ..
            } = bds.bds40.unwrap();
            assert_eq!(selected_altitude_fms.unwrap(), 3000);
            assert_eq!(selected_altitude_mcp.unwrap(), 3000);
            assert_relative_eq!(
                barometric_setting.unwrap(),
                1020.,
                max_relative = 1e-3
            );
        } else {
            unreachable!();
        }
    }
    #[test]
    fn test_invalid_bds40() {
        let bytes = hex!("a0000638fa81c10000000081a92f");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(bds.bds40, None);
        } else {
            unreachable!();
        }
    }
}
