use deku::prelude::*;
use serde::Serialize;

/**
 * ## Common usage GICB capability report (BDS 1,7)
 *
 * A bit when the corresponding register has a valid input that has been updated
 * at the required rate. This means that the same aircraft would respond with
 * different GICB reports due to the availability of the relevant data.
 *
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[serde(tag = "bds", rename = "17")]
pub struct CommonUsageGICBCapabilityReport {
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Extended squitter airborne position
    pub bds05: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Extended squitter surface position
    pub bds06: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Extended squitter status
    pub bds07: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Extended squitter identification and category
    pub bds08: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Extended squitter airborne velocity information
    pub bds09: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Extended squitter event-driven information
    pub bds0a: bool,

    #[deku(bits = "1", map = "fail_if_false")]
    #[serde(skip_serializing_if = "is_false")]
    /// Aircraft identification
    pub bds20: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Aircraft registration number
    pub bds21: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Selected vertical intention
    pub bds40: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Next waypoint identifier
    pub bds41: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Next waypoint position
    pub bds42: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Next waypoint information
    pub bds43: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Meteorological routine report
    pub bds44: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Meteorological hazard report
    pub bds45: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// VHF channel report
    pub bds48: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Track and turn report
    pub bds50: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Position coarse
    pub bds51: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Position fine
    pub bds52: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Air-referenced state vector
    pub bds53: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Waypoint 1
    pub bds54: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Waypoint 2
    pub bds55: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Waypoint 3
    pub bds56: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Quasi-static parameter monitoring
    pub bds5f: bool,

    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    /// Heading and speed report
    pub bds60: bool,

    #[deku(bits = "5")]
    #[serde(skip)]
    pub reserved: u8,

    #[deku(reader = "check_zeros(deku::reader)")]
    #[serde(skip)]
    pub check_flag: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn fail_if_false(value: bool) -> Result<bool, DekuError> {
    if value {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "BDS 2,0 is always valid in BDS 1,7".into(),
        ))
    }
}

fn check_zeros<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<bool, DekuError> {
    for i in 0..=3 {
        let value = u8::from_reader_with_ctx(
            reader,
            (
                deku::ctx::Endian::Big,
                deku::ctx::BitSize(if i == 0 { 3 } else { 8 }),
            ),
        )?;
        if value != 0 {
            return Err(DekuError::InvalidParam(
                "BDS 1,7 must have all final bits set to 0".into(),
            ));
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_valid_bds17() {
        let bytes = hex!("a0000638fa81c10000000081a92f");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(
                bds.bds17,
                Some(CommonUsageGICBCapabilityReport {
                    bds05: true,
                    bds06: true,
                    bds07: true,
                    bds08: true,
                    bds09: true,
                    bds0a: false,
                    bds20: true,
                    bds21: false,
                    bds40: true,
                    bds41: false,
                    bds42: false,
                    bds43: false,
                    bds44: false,
                    bds45: false,
                    bds48: false,
                    bds50: true,
                    bds51: true,
                    bds52: true,
                    bds53: false,
                    bds54: false,
                    bds55: false,
                    bds56: false,
                    bds5f: false,
                    bds60: true,
                    reserved: 0,
                    check_flag: true
                })
            );
        } else {
            unreachable!();
        }
    }
    #[test]
    fn test_invalid_bds17() {
        let bytes = hex!("a0001838201584f23468207cdfa5");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(bds.bds17, None);
        } else {
            unreachable!();
        }
    }
}
