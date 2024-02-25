extern crate alloc;

use alloc::fmt;
use deku::prelude::*;
use serde::Serialize;

/// Target State and Status (§2.2.3.2.7.1)
#[derive(Copy, Clone, Debug, Serialize, PartialEq, DekuRead)]
pub struct TargetStateAndStatusInformation {
    #[deku(bits = "2")] // bits 5..=6
    #[serde(skip)]
    pub subtype: u8, // must be equal to 1

    #[deku(pad_bits_before = "1")] // bit 7
    #[serde(rename = "source")]
    pub alt_source: AltSource, // bit 8

    #[deku(
        bits = "11",// bit 9..20
        endian = "big",
        map = "|altitude: u32| -> Result<_, DekuError> {
            Ok(
                if altitude > 1 {Some(((altitude - 1) * 32 + 16) / 100 * 100)}
                else {None}
            )
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_altitude: Option<u32>,

    #[deku(
        bits = "9", // bit 20..29
        endian = "big",
        map = "|qnh: u32| -> Result<_, DekuError> {
            if qnh == 0 { Ok(None) }
            else { Ok(Some(800.0 + ((qnh - 1) as f32) * 0.8))}
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barometric_setting: Option<f32>,

    #[deku(bits = "1")] // bit 29
    #[serde(skip)]
    pub heading_status: bool,

    #[deku(
        bits = "9",// bit 30..39
        endian = "big",
        map = "|heading: u16| -> Result<_, DekuError> {
            if *heading_status {Ok(Some(heading as f32 * 180.0 / 256.0))} 
            else {Ok(None)}
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_heading: Option<f32>,

    #[deku(bits = "4")]
    #[serde(rename = "NACp")]
    pub nacp: u8,

    #[deku(bits = "1")]
    /// Barometric Altitude Integrity Code (NIC baro)
    // is the baroaltitude crosschecked with another source of pressure?
    #[serde(skip)]
    pub nicbaro: bool,

    #[deku(bits = "2")]
    #[serde(skip)] // per sample
    pub sil: u8,

    #[deku(bits = "1")]
    #[serde(skip)]
    pub mcp_fcp_status: bool,

    #[deku(
        bits = "1",
        map = "|val: bool| -> Result<_, DekuError> {
            if *mcp_fcp_status {Ok(Some(val))} else {Ok(None)}
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autopilot: Option<bool>,
    #[deku(
        bits = "1",
        map = "|val: bool| -> Result<_, DekuError> {
            if *mcp_fcp_status {Ok(Some(val))} else {Ok(None)}
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vnav_mode: Option<bool>,
    #[deku(
        bits = "1",
        map = "|val: bool| -> Result<_, DekuError> {
            if *mcp_fcp_status {Ok(Some(val))} else {Ok(None)}
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_hold: Option<bool>,
    #[deku(bits = "1")]
    #[serde(skip)]
    // Not so sure what this is...
    pub imf: bool,
    #[deku(
        bits = "1",
        map = "|val: bool| -> Result<_, DekuError> {
            if *mcp_fcp_status {Ok(Some(val))} else {Ok(None)}
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approach_mode: Option<bool>,
    #[deku(bits = "1")] // bit 52, ALWAYS VALID!
    pub tcas_operational: bool,
    #[deku(
        bits = "1",
        map = "|val: bool| -> Result<_, DekuError> {
            if *mcp_fcp_status {Ok(Some(val))} else {Ok(None)}
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[deku(pad_bits_after = "2")] // reserved
    pub lnav_mode: Option<bool>,
}

impl fmt::Display for TargetStateAndStatusInformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Target state and status (BDS 6,2)")?;
        if let Some(sel_alt) = &self.selected_altitude {
            writeln!(
                f,
                "  Selected alt:  {} ft {}",
                sel_alt, &self.alt_source
            )?;
        }
        if let Some(sel_hdg) = &self.selected_heading {
            writeln!(f, "  Selected hdg:  {:.1}°", sel_hdg)?;
        }
        if let Some(qnh) = &self.barometric_setting {
            writeln!(f, "  QNH:           {:.1} mbar", qnh)?;
        }
        if self.mcp_fcp_status {
            write!(f, "  Mode:         ")?;
            if let Some(value) = self.autopilot {
                if value {
                    write!(f, " autopilot")?;
                }
            }
            if let Some(value) = self.vnav_mode {
                if value {
                    write!(f, " VNAV")?;
                }
            }
            if let Some(value) = self.lnav_mode {
                if value {
                    write!(f, " LNAV")?;
                }
            }
            if let Some(value) = self.alt_hold {
                if value {
                    write!(f, " alt_hold")?;
                }
            }
            if let Some(value) = self.approach_mode {
                if value {
                    write!(f, " approach")?;
                }
            }
            writeln!(f, "")?;
        }
        writeln!(
            f,
            "  TCAS:          {}",
            if self.tcas_operational { "on" } else { "off" }
        )
    }
}

#[derive(Copy, Clone, Debug, Serialize, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "1")]
pub enum AltSource {
    #[deku(id = "0")]
    #[serde(rename = "MCP/FCU")]
    MCP,

    #[deku(id = "1")]
    FMS,
}
impl fmt::Display for AltSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::MCP => write!(f, "MCP/FCU"),
            Self::FMS => write!(f, "FMS"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::adsb::ME::BDS62;
    use crate::decode::{Message, DF::ADSB};
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_surface_position() {
        let bytes = hex!("8DA05629EA21485CBF3F8CADAEEB");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ADSB(adsb_msg) = msg.df {
            if let BDS62(state) = adsb_msg.message {
                assert_eq!(state.selected_altitude, Some(17000));
                assert_eq!(state.alt_source, AltSource::MCP);
                assert_eq!(state.barometric_setting, Some(1012.8));
                if let Some(sel_hdg) = state.selected_heading {
                    assert_relative_eq!(sel_hdg, 66.8, max_relative = 1e-2);
                } else {
                    unreachable!()
                }
                assert_eq!(state.mcp_fcp_status, true);
                assert_eq!(state.autopilot, Some(true));
                assert_eq!(state.vnav_mode, Some(true));
                assert_eq!(state.lnav_mode, Some(true));
                assert_eq!(state.alt_hold, Some(false));
                assert_eq!(state.approach_mode, Some(false));
                assert_eq!(state.tcas_operational, true);
            }
            return;
        }
        unreachable!();
    }

    #[test]
    fn test_format_groundspeed() {
        let bytes = hex!("8DA05629EA21485CBF3F8CADAEEB");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(
            format!("{msg}"),
            r#" DF17. Extended Squitter
  Address:       a05629
  Air/Ground:    airborne
  Target state and status (BDS 6,2)
  Selected alt:  17000 ft MCP/FCU
  Selected hdg:  66.8°
  QNH:           1012.8 mbar
  Mode:          autopilot VNAV LNAV
  TCAS:          on
"#
        )
    }
}
