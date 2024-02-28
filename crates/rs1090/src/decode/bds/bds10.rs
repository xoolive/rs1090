use deku::prelude::*;
use serde::Serialize;

/**
 * ## Data link Capability Report (BDS 1,0)
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct DataLinkCapability {
    #[deku(bits = "8", map = "fail_if_not10")]
    #[serde(skip)]
    pub bds: u8,
    #[deku(bits = "1")]
    #[deku(pad_bits_after = "5")] // reserved
    pub continuation_flag: bool,
    #[deku(bits = "1")]
    pub overlay_command_capability: bool,
    #[deku(bits = "1")]
    pub acas: bool,
    #[deku(bits = "7")]
    pub mode_s_subnetwork_version_number: u8,
    #[deku(bits = "1")]
    pub transponder_enhanced_protocol_indicator: bool,
    #[deku(bits = "1")]
    pub mode_s_specific_services_capability: bool,
    #[deku(bits = "3")]
    pub uplink_elm_average_throughput_capability: u8,
    #[deku(bits = "4")]
    pub downlink_elm: u8,
    #[deku(bits = "1")]
    pub aircraft_identification_capability: bool,
    #[deku(bits = "1")]
    pub squitter_capability_subfield: bool,
    #[deku(bits = "1")]
    pub surveillance_identifier_code: bool,
    #[deku(bits = "1")]
    pub common_usage_gicb_capability_report: bool,
    #[deku(bits = "4")]
    pub reserved_acas: u8,
    pub bit_array: u16,
}

fn fail_if_not10(value: u8) -> Result<u8, DekuError> {
    if value == 0x10 {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "First bits must be 0x10 in BDS 1,0".to_string(),
        ))
    }
}
