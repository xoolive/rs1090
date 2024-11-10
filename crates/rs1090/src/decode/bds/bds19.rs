use deku::prelude::*;
use serde::Serialize;

/**
 * ## GICB capability report (2 of 5) (BDS 1,9)
 *
 * A bit when the corresponding register has a valid input that has been updated
 * at the required rate. This means that the same aircraft would respond with
 * different GICB reports due to the availability of the relevant data.
 *
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[serde(tag = "bds", rename = "19")]
pub struct GICBCapabilityReportPart2 {
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds70: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds6f: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds6e: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds6d: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds6c: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds6b: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds6a: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds69: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds68: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds67: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds66: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds65: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds64: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds63: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds62: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds61: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds60: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds5f: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds5e: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds5d: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds5c: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds5b: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds5a: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds59: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds58: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds57: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds56: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds55: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds54: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds53: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds52: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds51: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds50: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds4f: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds4e: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds4d: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds4c: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds4b: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds4a: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds49: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds48: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds47: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds46: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds45: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds44: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds43: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds42: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds41: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds40: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds3f: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds3e: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds3d: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds3c: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds3b: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds3a: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds39: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn fail_if_true(value: bool) -> Result<bool, DekuError> {
    if value {
        Err(DekuError::Assertion(
            "Field is most probably false in BDS 1,9".into(),
        ))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_valid_bds19() {
        let bytes = hex!("a00001ba00018003800080000000");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert!(bds.bds19.is_some());
        } else {
            unreachable!();
        }
    }
}
