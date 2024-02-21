use deku::prelude::*;
use serde::Serialize;

/// Target State and Status (§2.2.3.2.7.1)
#[derive(Copy, Clone, Debug, Serialize, PartialEq, DekuRead)]
pub struct TargetStateAndStatusInformation {
    // TODO Support Target State and Status defined in DO-260A, ADS-B Version=1
    // TODO Support reserved 2..=3
    #[deku(bits = "2")]
    pub subtype: u8,
    #[deku(bits = "1")]
    pub is_fms: bool,
    #[deku(
        bits = "12",
        endian = "big",
        map = "|altitude: u32| -> Result<_, DekuError> {Ok(if altitude > 1 {(altitude - 1) * 32} else {0} )}"
    )]
    pub selected_altitude: u32,
    #[deku(
        bits = "9",
        endian = "big",
        map = "|qnh: u32| -> Result<_, DekuError> {if qnh == 0 { Ok(0.0) } else { Ok(800.0 + ((qnh - 1) as f32) * 0.8)}}"
    )]
    pub qnh: f32,
    #[deku(bits = "1")]
    pub is_heading: bool,
    #[deku(
        bits = "9",
        endian = "big",
        map = "|heading: u16| -> Result<_, DekuError> {Ok(heading as f32 * 180.0 / 256.0)}"
    )]
    pub selected_heading: f32,
    #[deku(bits = "4")]
    pub nacp: u8,
    #[deku(bits = "1")]
    pub nicbaro: u8,
    #[deku(bits = "2")]
    pub sil: u8,
    #[deku(bits = "1")]
    pub mode_validity: bool,
    #[deku(bits = "1")]
    pub autopilot: bool,
    #[deku(bits = "1")]
    pub vnav_mode: bool,
    #[deku(bits = "1")]
    pub alt_hold: bool,
    #[deku(bits = "1")]
    pub imf: bool,
    #[deku(bits = "1")]
    pub approach: bool,
    #[deku(bits = "1")]
    pub tcas: bool,
    #[deku(bits = "1")]
    #[deku(pad_bits_after = "2")] // reserved
    pub lnav_mode: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::adsb::Typecode::TargetStateAndStatusInformation;
    use crate::decode::{Message, DF::ADSB};
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_surface_position() {
        let bytes = hex!("8DA05629EA21485CBF3F8CADAEEB");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ADSB(adsb_msg) = msg.df {
            if let TargetStateAndStatusInformation(state) = adsb_msg.message {
                assert_eq!(state.selected_altitude, 16992);
                assert_eq!(state.is_fms, false);
                assert_eq!(state.qnh, 1012.8);
                assert_eq!(state.is_heading, true);
                assert_relative_eq!(
                    state.selected_heading,
                    66.8,
                    max_relative = 1e-2
                );
                assert_eq!(state.autopilot, true);
                assert_eq!(state.vnav_mode, true);
                assert_eq!(state.lnav_mode, true);
                assert_eq!(state.alt_hold, false);
                assert_eq!(state.approach, false);
                assert_eq!(state.tcas, true);

                return;
            }
        }
        unreachable!();
    }
}
