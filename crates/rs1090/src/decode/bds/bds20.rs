use super::bds08;
use deku::prelude::*;
use serde::{Deserialize, Serialize};

/**
 * ## Aircraft Identification (BDS 2,0)
 *
 * Comm-B message providing aircraft callsign/identification.  
 * Per ICAO Doc 9871 Table A-2-32: BDS code 2,0 — Aircraft identification
 *
 * Purpose: To report aircraft identification to the ground.
 *
 * Message Structure (56 bits):
 * | BDS  | CHAR1 | CHAR2 | CHAR3 | CHAR4 | CHAR5 | CHAR6 | CHAR7 | CHAR8 |
 * |------|-------|-------|-------|-------|-------|-------|-------|-------|
 * | 8    | 6     | 6     | 6     | 6     | 6     | 6     | 6     | 6     |
 *
 * Field Encoding per ICAO Doc 9871:
 *
 * **BDS Code** (bits 1-8):
 *   - Fixed value: 0x20 (0010 0000 binary = 2,0 hexadecimal)
 *   - Identifies this register as BDS 2,0
 *
 * **Aircraft Identification** (bits 9-56):
 *   - 8 characters, 6 bits each
 *   - Character encoding per ICAO Annex 10, Vol IV, Table 3-7
 *   - Same encoding as used in Extended Squitter BDS 0,8
 *   - Valid characters: A-Z, 0-9, and space
 *   - Character set (6-bit encoding):
 *     * 000000 = invalid/reserved (not a space)
 *     * 000001-011010 = A-Z
 *     * 110000-111001 = 0-9
 *     * 100000 = space
 *
 * Content Rules per ICAO Doc 9871:
 * - The aircraft identification shall be that employed in the flight plan
 * - When no flight plan is available, the registration marking shall be used
 * - Characters 1-8 are also used by the Extended Squitter application (BDS 0,8)
 * - Data may be input from sources other than Mode S ADLP
 *
 * Capability Indication:
 * - Support indicated by bit 33 in register 1,0 (0x10)
 * - Also indicated in registers 1,7 (0x17) and 1,8 (0x18)
 *
 * Note: This is the Comm-B version of aircraft identification.
 * The Extended Squitter version is BDS 0,8 with identical character encoding.
 *
 * Additional implementation guidelines: ICAO Doc 9871 §D.2.4.3
 */

#[derive(
    Debug, PartialEq, Serialize, Deserialize, DekuRead, Clone, Default,
)]
#[serde(tag = "bds", rename = "20")]
pub struct AircraftIdentification {
    /// BDS Code (bits 1-8): Per ICAO Doc 9871 Table A-2-32  
    /// Fixed identifier for this register.  
    /// Must be 0x20 (0010 0000 binary = 2,0 hexadecimal).
    #[deku(bits = "8", map = "fail_if_not20")]
    #[serde(skip)]
    pub bds: u8,

    /// Aircraft Identification (bits 9-56): Per ICAO Doc 9871 Table A-2-32  
    /// Callsign or registration marking (8 characters, 6 bits each).  
    /// Character encoding per ICAO Annex 10, Vol IV, Table 3-7.  
    /// Valid characters: A-Z (1-26), 0-9 (48-57), space (32).  
    /// Content:
    ///   - Aircraft identification from flight plan (when available)
    ///   - Aircraft registration marking (when no flight plan)
    ///
    /// Same encoding as Extended Squitter BDS 0,8.
    /// Empty identification or reserved character codes leave this field
    /// unavailable, without invalidating the BDS20 register hypothesis.
    #[deku(reader = "callsign_read(deku::reader)")]
    #[serde(
        default,
        deserialize_with = "deserialize_callsign",
        skip_serializing_if = "Option::is_none"
    )]
    pub callsign: Option<String>,
}

fn callsign_read<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<String>, DekuError> {
    let callsign = bds08::callsign_read(reader)?;
    Ok(available_callsign(callsign))
}

fn available_callsign(callsign: String) -> Option<String> {
    (!callsign.trim().is_empty() && !callsign.contains('#')).then_some(callsign)
}

fn deserialize_callsign<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?
        .and_then(available_callsign))
}

fn fail_if_not20(value: u8) -> Result<u8, DekuError> {
    if value == 0x20 {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "First bits must be 0x20 in BDS 2,0".into(),
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
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(
                bds.bds20,
                Some(super::AircraftIdentification {
                    bds: 32,
                    callsign: Some("EXS2MF".to_string())
                })
            );
            assert_eq!(
                serde_json::to_value(&bds).unwrap()["bds20"]["callsign"],
                "EXS2MF"
            );
        } else {
            unreachable!();
        }
    }
    #[test]
    fn test_unavailable_bds20_retains_register_with_no_callsign() {
        // These synthetic frames exercise payload inference, not RF CRC
        // validation: all-zero identification, all spaces, and a reserved
        // character mixed with otherwise valid identification.
        for bytes in [
            hex!("a0001838200000000000007cdfa5"),
            hex!("a0001838208208208208207cdfa5"),
            hex!("a0001838200184f23468207cdfa5"),
        ] {
            let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
            if let CommBAltitudeReply { bds, .. } = msg.df {
                let identification =
                    bds.bds20.as_ref().expect("BDS20 header remains valid");
                assert_eq!(identification.callsign, None);
                let json = serde_json::to_value(&bds).unwrap();
                assert_eq!(json["bds20"], serde_json::json!({"bds":"20"}));
                let restored: super::AircraftIdentification =
                    serde_json::from_value(json["bds20"].clone()).unwrap();
                assert_eq!(restored.callsign, None);
                assert_eq!(
                    serde_json::to_value(restored).unwrap(),
                    json["bds20"]
                );
            } else {
                unreachable!();
            }
        }
    }

    #[test]
    fn test_legacy_json_unavailable_callsigns_are_normalized() {
        for callsign in [
            serde_json::Value::Null,
            serde_json::json!(""),
            serde_json::json!("   "),
            serde_json::json!("########"),
            serde_json::json!("F#HRBE"),
        ] {
            let identification: super::AircraftIdentification =
                serde_json::from_value(
                    serde_json::json!({"bds":"20", "callsign":callsign}),
                )
                .unwrap();
            assert!(identification.callsign.is_none());
            assert_eq!(
                serde_json::to_value(identification).unwrap(),
                serde_json::json!({"bds":"20"})
            );
        }
        let valid: super::AircraftIdentification = serde_json::from_value(
            serde_json::json!({"bds":"20", "callsign":"EXS2MF"}),
        )
        .unwrap();
        assert_eq!(valid.callsign.as_deref(), Some("EXS2MF"));
    }

    #[test]
    #[cfg(feature = "bds-infer")]
    fn test_zero_identification_has_a_structural_weather_alternative() {
        use crate::decode::bds::{decode_bds, infer_bds, DecodedBds};
        let zero_identification = hex!("20000000000000");
        assert!(decode_bds(&zero_identification, 0x44).is_ok());
        assert!(decode_bds(&hex!("20820820820820"), 0x44).is_err());
        let selected = infer_bds(&zero_identification, None);
        assert_eq!(selected.len(), 1);
        assert!(
            matches!(&selected[0], DecodedBds::Bds20(identification) if identification.callsign.is_none())
        );
    }

    #[test]
    fn test_invalid_bds20() {
        let bytes = hex!("a800178d10010080f50000d5893c");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let CommBIdentityReply { bds, .. } = msg.df {
            assert_eq!(bds.bds20, None);
        } else {
            unreachable!();
        }
    }
}
