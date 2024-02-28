use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::Serialize;

/**
 * ## Common usage GICB capability report (BDS 1,7)
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct GICBCapabilityReport {
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds05: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds06: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds07: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds08: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds09: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds0a: bool,
    #[deku(bits = "1", map = "fail_if_false")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds20: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds21: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds40: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds41: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds42: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds43: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds44: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds45: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds48: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds50: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds51: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds52: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds53: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds54: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds55: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds56: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds5f: bool,
    #[deku(bits = "1")]
    #[serde(skip_serializing_if = "is_false")]
    pub bds60: bool,
    #[deku(reader = "check_zeros(deku::rest)")]
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
            "BDS 2,0 is always valid in BDS 1,7".to_string(),
        ))
    }
}

fn check_zeros(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, bool), DekuError> {
    let mut inside_rest = rest;
    for _ in 0..=3 {
        let (for_rest, value) = u8::read(
            inside_rest,
            (deku::ctx::Endian::Big, deku::ctx::BitSize(8)),
        )?;
        if value != 0 {
            return Err(DekuError::InvalidParam(
                "BDS 1,7 must have all final bits set to 0".to_string(),
            ));
        }
        inside_rest = for_rest;
    }
    Ok((rest, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_valid_bds17() {
        let bytes = hex!("a0000638fa81c10000000081a92f");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(
                bds.bds17,
                Some(GICBCapabilityReport {
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
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(bds.bds17, None);
        } else {
            unreachable!();
        }
    }
}
