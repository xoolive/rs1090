use deku::prelude::*;
use serde::Serialize;

/**
 * ## GICB capability report (1 of 5) (BDS 1,8)
 *
 * A bit when the corresponding register has a valid input that has been updated
 * at the required rate. This means that the same aircraft would respond with
 * different GICB reports due to the availability of the relevant data.
 *
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[serde(tag = "bds", rename = "18")]
pub struct GICBCapabilityReportPart1 {
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds38: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds37: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds36: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds35: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds34: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds33: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds32: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds31: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds30: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds2f: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds2e: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds2d: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds2c: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds2b: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds2a: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds29: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds28: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds27: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds26: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds25: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds24: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds23: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds22: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds21: bool,
    #[deku(bits = "1", map = "fail_if_false")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds20: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds1f: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds1e: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds1d: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds1c: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds1b: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds1a: bool,
    #[deku(bits = "1", map = "fail_if_false")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds19: bool,
    #[deku(bits = "1", map = "fail_if_false")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds18: bool,
    #[deku(bits = "1", map = "fail_if_false")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds17: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds16: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds15: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds14: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds13: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds12: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds11: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds10: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds0f: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds0e: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds0d: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds0c: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds0b: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds0a: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds09: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds08: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds07: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds06: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds05: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds04: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds03: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds02: bool,
    #[deku(bits = "1", map = "fail_if_true")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds01: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn fail_if_false(value: bool) -> Result<bool, DekuError> {
    if value {
        Ok(value)
    } else {
        Err(DekuError::Assertion(
            "BDS 1,7, 1,8, 1,9 and 2,0 are always valid in BDS 1,8".into(),
        ))
    }
}

fn fail_if_true(value: bool) -> Result<bool, DekuError> {
    if value {
        Err(DekuError::Assertion(
            "Field is most probably false in BDS 1,8".into(),
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
    fn test_valid_bds18() {
        let bytes = hex!("a000019b0080008fc083f0000000");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert!(bds.bds18.is_some());
        } else {
            unreachable!();
        }
    }
}
