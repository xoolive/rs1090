use deku::prelude::*;
use serde::Serialize;

/**
 * ## Data link Capability Report (BDS 1,0)
 *
 * This message is designed to report the data link capability of the installed
 * Mode S transponder.
 *
 * In the data link capability report, the first eight bits indicate the BDS
 * number, which is 1,0, or 0001 0000 in binary format.
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct DataLinkCapability {
    #[deku(bits = "8", map = "fail_if_not10")]
    #[serde(skip)]
    /// The first eight bits indicate the BDS code 1000 0000 (1,0 in hexadecimal).
    pub bds: u8,

    #[deku(bits = "1")]
    /// Configuration flag
    pub config: bool,

    #[deku(bits = "5", map = "fail_if_not0")]
    #[serde(skip)]
    pub reserved: u8,

    #[deku(bits = "1")]
    /// Overlay command capacity (OVC) indicates whether the transponder
    /// supports BDS overlay (Data Parity).
    pub ovc: bool,

    #[deku(bits = "1")]
    /// ACAS operating
    pub acas: bool,

    #[deku(bits = "7")]
    /// Mode S subnetwork version number
    ///
    /// - 0:  Subnetwork not available
    /// - 1:  ICAO Doc 9688 (1996)
    /// - 2:  ICAO Doc 9688 (1998)
    /// - 3:  ICAO Annex 10, Vol III, Amdt 77
    /// - 4:  ICAO Doc 9871 (Ed 1), RTCA DO-181D, EUROCAE ED-73C
    /// - 5:  ICAO Doc 9871 (Ed 2), RTCA DO-181E, EUROCAE ED-73E
    /// - >5: Reserved for future use
    pub subnet: u8,

    #[deku(bits = "1")]
    /// Transponder enhancend protocol indicator (Level 5 if true)
    pub level5: bool,

    #[deku(bits = "1")]
    /// Mode S specific services capability
    pub mode_s: bool,

    #[deku(bits = "3")]
    #[serde(skip)]
    /// Uplink ELM average throughput capacity
    pub uplink: u8,

    #[deku(bits = "4")]
    #[serde(skip)]
    /// Downlink ELM average throughput
    pub downlink: u8,

    #[deku(bits = "1")]
    /// Aircraft identification capability indicates availability of
    /// identification (callsign)
    pub identification: bool,

    #[deku(bits = "1")]
    /// Squitter capability subfield. True if both
    /// BDS 0,5 and 0,6 registers have been updated in the past 9 to 11 seconds.
    pub squitter: bool,

    #[deku(bits = "1")]
    /// Surveillance identifier code (SIC) capability
    pub sic: bool,

    #[deku(bits = "1")]
    /// True every time the GICB capacity report (BDS 1,7) is changed.
    pub gicb: bool,

    #[deku(bits = "1")]
    /// Hybrid surveillance fitted and operational
    pub acas_hybrid: bool,

    #[deku(bits = "1")]
    /// ACAS generating TAs and RAs (false means TA only)
    pub acas_ra: bool,

    #[deku(bits = "2")]
    #[serde(skip)]
    /// RTCA/DO-185 (0), RTCA/DO-185-A (1), RTCA/DO-185-B (2)
    pub acas_rtca: u8,

    /// Data terminal equipment (DTE) status
    #[deku(bits = "16")]
    pub dte: u16,
}

fn fail_if_not0(value: u8) -> Result<u8, DekuError> {
    if value == 0 {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "Reserved bits must be 0 in BDS 1,0".to_string(),
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_valid_bds10() {
        let bytes = hex!("a800178d10010080f50000d5893c");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBIdentityReply { bds, .. } = msg.df {
            assert_eq!(
                bds.bds10,
                Some(DataLinkCapability {
                    bds: 16,
                    config: false,
                    reserved: 0,
                    ovc: false,
                    acas: true,
                    subnet: 0,
                    level5: false,
                    mode_s: true,
                    uplink: 0,
                    downlink: 0,
                    identification: true,
                    squitter: true,
                    sic: true,
                    gicb: true,
                    acas_hybrid: false,
                    acas_ra: true,
                    acas_rtca: 1,
                    dte: 0
                })
            );
        } else {
            unreachable!();
        }
    }
    #[test]
    fn test_invalid_bds10() {
        let bytes = hex!("a0001838201584f23468207cdfa5");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(bds.bds10, None);
        } else {
            unreachable!();
        }
    }
}
