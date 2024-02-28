use super::bds08;
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Aircraft identification (BDS 2,0)
 *
 * Similar to an ADS-B aircraft identification message, the callsign of an
 * aircraft can be decoded from BDS 2,0 messages.
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct AircraftIdentification {
    #[deku(bits = "8", map = "fail_if_not20")]
    #[serde(skip)]
    /// The first eight bits indicate the BDS code 0010 0000 (2,0 in hexadecimal).
    pub bds: u8,

    #[deku(reader = "bds08::callsign_read(deku::rest)")]
    pub callsign: String,
}

fn fail_if_not20(value: u8) -> Result<u8, DekuError> {
    if value == 0x20 {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "First bits must be 0x20 in BDS 2,0".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_valid_bds20() {
        let bytes = hex!("a0001838201584f23468207cdfa5");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(
                bds.bds20,
                Some(super::AircraftIdentification {
                    bds: 32,
                    callsign: "EXS2MF".to_string()
                })
            );
        } else {
            unreachable!();
        }
    }
    #[test]
    fn test_invalid_bds20() {
        let bytes = hex!("a800178d10010080f50000d5893c");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBIdentityReply { bds, .. } = msg.df {
            assert_eq!(bds.bds20, None);
        } else {
            unreachable!();
        }
    }
}
