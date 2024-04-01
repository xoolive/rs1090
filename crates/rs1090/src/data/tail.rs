use once_cell::sync::Lazy;

// Comment from the flightaware implementation of dump1090
// https://github.com/flightaware/dump1090/blob/master/public_html/registrations.js
//
// Various reverse-engineered versions of the allocation algorithms
// used by different countries to allocate 24-bit ICAO addresses based
// on the aircraft registration.
//
// These were worked out by looking at the allocation patterns and
// working backwards to an algorithm that generates that pattern,
// spot-checking aircraft to see if it worked.

const LIMITED_ALPHABET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ"; // 24 chars; no I, O
const FULL_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 26 chars

static NUMERIC_MAPPINGS: Lazy<Vec<NumericMapping>> = Lazy::new(|| {
    vec![
        NumericMapping::new(0x140000, 0, 100000, String::from("RA-00000")),
        NumericMapping::new(0x0B03E8, 1000, 1000, String::from("CU-T0000")),
    ]
});

static STRIDE_MAPPINGS: Lazy<Vec<StrideMapping>> = Lazy::new(|| {
    vec![
        StrideMapping::new(
            0x008011,
            26 * 26,
            26,
            String::from("ZS-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x390000,
            1024,
            32,
            String::from("F-G"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x398000,
            1024,
            32,
            String::from("F-H"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x3C4421,
            1024,
            32,
            String::from("D-A"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("OZZ"),
        ),
        StrideMapping::new(
            0x3C0001,
            26 * 26,
            26,
            String::from("D-A"),
            String::from(FULL_ALPHABET),
            String::from("PAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x3C8421,
            1024,
            32,
            String::from("D-B"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("OZZ"),
        ),
        StrideMapping::new(
            0x3C2001,
            26 * 26,
            26,
            String::from("D-B"),
            String::from(FULL_ALPHABET),
            String::from("PAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x3CC000,
            26 * 26,
            26,
            String::from("D-C"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x3D04A8,
            26 * 26,
            26,
            String::from("D-E"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x3D4950,
            26 * 26,
            26,
            String::from("D-F"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x3D8DF8,
            26 * 26,
            26,
            String::from("D-G"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x3DD2A0,
            26 * 26,
            26,
            String::from("D-H"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x3E1748,
            26 * 26,
            26,
            String::from("D-I"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x448421,
            1024,
            32,
            String::from("OO-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x458421,
            1024,
            32,
            String::from("OY-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x460000,
            26 * 26,
            26,
            String::from("OH-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x468421,
            1024,
            32,
            String::from("SX-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x490421,
            1024,
            32,
            String::from("CS-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x4A0421,
            1024,
            32,
            String::from("YR-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x4B8421,
            1024,
            32,
            String::from("TC-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x740421,
            1024,
            32,
            String::from("JY-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x760421,
            1024,
            32,
            String::from("AP-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x768421,
            1024,
            32,
            String::from("9V-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x778421,
            1024,
            32,
            String::from("YK-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0x7C0000,
            36 * 36,
            36,
            String::from("VH-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0xC00001,
            26 * 26,
            26,
            String::from("C-F"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0xC044A9,
            26 * 26,
            26,
            String::from("C-G"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        StrideMapping::new(
            0xE01041,
            4096,
            64,
            String::from("LV-"),
            String::from(FULL_ALPHABET),
            String::from("AAA"),
            String::from("ZZZ"),
        ),
        // Add other mappings here...
    ]
});

/// Handles 3-letter suffixes assigned with a regular pattern
struct StrideMapping {
    /// first hexid of range
    start: u32,
    /// major stride (interval between different first letters)
    s1: u32,
    /// minor stride (interval between different second letters)
    s2: u32,
    /// the registration prefix
    prefix: String,
    /// default to FULL_ALPHABET
    alphabet: String,
    // the suffix to use at the start of the range (default: AAA)
    // first: String,
    // the last valid suffix in the range (default: ZZZ)
    // last: String,
    offset: u32,
    end: u32,
}

impl StrideMapping {
    fn new(
        start: u32,
        s1: u32,
        s2: u32,
        prefix: String,
        alphabet: String,
        first: String,
        last: String,
    ) -> Self {
        let offset = {
            let c1 = alphabet
                .chars()
                .position(|c| c == first.chars().nth(0).unwrap())
                .unwrap_or(0) as u32;
            let c2 = alphabet
                .chars()
                .position(|c| c == first.chars().nth(1).unwrap())
                .unwrap_or(0) as u32;
            let c3 = alphabet
                .chars()
                .position(|c| c == first.chars().nth(2).unwrap())
                .unwrap_or(0) as u32;
            c1 * s1 + c2 * s2 + c3
        };

        let end = {
            let c1 = alphabet
                .chars()
                .position(|c| c == last.chars().nth(0).unwrap())
                .unwrap_or(alphabet.len() - 1) as u32;
            let c2 = alphabet
                .chars()
                .position(|c| c == last.chars().nth(1).unwrap())
                .unwrap_or(alphabet.len() - 1) as u32;
            let c3 = alphabet
                .chars()
                .position(|c| c == last.chars().nth(2).unwrap())
                .unwrap_or(alphabet.len() - 1) as u32;
            start - offset + c1 * s1 + c2 * s2 + c3
        };

        StrideMapping {
            start,
            s1,
            s2,
            prefix,
            alphabet,
            //first,
            //last,
            offset,
            end,
        }
    }
}

pub fn stride_reg(hexid: u32) -> Option<String> {
    for mapping in STRIDE_MAPPINGS.iter() {
        if hexid >= mapping.start && hexid <= mapping.end {
            let mut offset = hexid - mapping.start + mapping.offset;

            let i1 = (offset / mapping.s1) as usize;
            offset %= mapping.s1;
            let i2 = (offset / mapping.s2) as usize;
            offset %= mapping.s2;
            let i3 = offset as usize;

            if i1 < mapping.alphabet.len()
                && i2 < mapping.alphabet.len()
                && i3 < mapping.alphabet.len()
            {
                return Some(format!(
                    "{}{}{}{}",
                    mapping.prefix,
                    mapping.alphabet.chars().nth(i1).unwrap(),
                    mapping.alphabet.chars().nth(i2).unwrap(),
                    mapping.alphabet.chars().nth(i3).unwrap()
                ));
            }
        }
    }
    None
}

struct NumericMapping {
    /// start hexid in range
    start: u32,
    /// first numeric registration
    first: u32,
    // number of numeric registrations
    //count: u32,
    /// registration template, trailing characters are replaced with the numeric registration
    template: String,
    end: u32,
}

impl NumericMapping {
    fn new(start: u32, first: u32, count: u32, template: String) -> Self {
        let end = start + count - 1;
        NumericMapping {
            start,
            first,
            template,
            end,
        }
    }
}

pub fn numeric_reg(hexid: u32) -> Option<String> {
    for mapping in NUMERIC_MAPPINGS.iter() {
        if hexid >= mapping.start && hexid <= mapping.end {
            let reg = (hexid - mapping.start + mapping.first).to_string();
            return Some(format!(
                "{}{}",
                &mapping.template[..(mapping.template.len() - reg.len())],
                reg
            ));
        }
    }
    None
}

// South Korea
pub fn hl_reg(hexid: u32) -> Option<String> {
    if (0x71BA00..=0x71BF99).contains(&hexid) {
        return Some(format!("HL{:X}", hexid - 0x71BA00 + 0x7200));
    }

    if (0x71C000..=0x71C099).contains(&hexid) {
        return Some(format!("HL{:X}", hexid - 0x71C000 + 0x8000));
    }

    if (0x71C200..=0x71C299).contains(&hexid) {
        return Some(format!("HL{:X}", hexid - 0x71C200 + 0x8200));
    }

    None
}

// Japan
pub fn ja_reg(hexid: u32) -> Option<String> {
    let offset = hexid.wrapping_sub(0x840000);
    if offset >= 229840 {
        return None;
    }

    let mut reg = String::from("JA");

    let digit1 = offset / 22984;
    if digit1 > 9 {
        return None;
    }
    reg.push_str(&digit1.to_string());
    let offset = offset % 22984;

    let digit2 = offset / 916;
    if digit2 > 9 {
        return None;
    }
    reg.push_str(&digit2.to_string());
    let mut offset = offset % 916;

    if offset < 340 {
        let digit3 = offset / 34;
        reg.push_str(&digit3.to_string());
        offset %= 34;

        if offset < 10 {
            return Some(reg + &offset.to_string());
        }

        offset -= 10;
        return Some(
            reg + &LIMITED_ALPHABET
                .chars()
                .nth(offset as usize)
                .unwrap()
                .to_string(),
        );
    }

    offset -= 340;
    let letter3 = offset / 24;
    Some(
        reg + &LIMITED_ALPHABET
            .chars()
            .nth(letter3 as usize)
            .unwrap()
            .to_string()
            + &LIMITED_ALPHABET
                .chars()
                .nth((offset % 24) as usize)
                .unwrap()
                .to_string(),
    )
}

fn n_letters(mut rem: u32) -> String {
    if rem == 0 {
        return String::new();
    }

    rem -= 1;
    format!(
        "{}{}",
        LIMITED_ALPHABET.chars().nth((rem / 25) as usize).unwrap(),
        n_letter(rem % 25)
    )
}

fn n_letter(mut rem: u32) -> String {
    if rem == 0 {
        return String::new();
    }

    rem -= 1;
    LIMITED_ALPHABET
        .chars()
        .nth(rem as usize)
        .unwrap()
        .to_string()
}

// United States
pub fn n_reg(hexid: u32) -> Option<String> {
    let offset = hexid.wrapping_sub(0xA00001);
    if offset >= 915399 {
        return None;
    }

    let digit1 = (offset / 101711) + 1;
    let mut reg = format!("N{}", digit1);
    let mut offset = offset % 101711;

    if offset <= 600 {
        return Some(reg + &n_letters(offset));
    }

    offset -= 601;

    let digit2 = offset / 10111;
    reg.push_str(&digit2.to_string());
    let mut offset = offset % 10111;

    if offset <= 600 {
        return Some(reg + &n_letters(offset));
    }

    offset -= 601;

    let digit3 = offset / 951;
    reg.push_str(&digit3.to_string());
    let mut offset = offset % 951;

    if offset <= 600 {
        return Some(reg + &n_letters(offset));
    }

    offset -= 601;

    let digit4 = offset / 35;
    reg.push_str(&digit4.to_string());
    let mut offset = offset % 35;

    if offset <= 24 {
        return Some(reg + &n_letter(offset));
    }

    offset -= 25;
    Some(reg + &offset.to_string())
}

pub fn tail(hexid: u32) -> Option<String> {
    if let Some(n_reg) = n_reg(hexid) {
        return Some(n_reg);
    }
    if let Some(ja_reg) = ja_reg(hexid) {
        return Some(ja_reg);
    }
    if let Some(hl_reg) = hl_reg(hexid) {
        return Some(hl_reg);
    }
    if let Some(numeric_reg) = numeric_reg(hexid) {
        return Some(numeric_reg);
    }
    if let Some(stride_reg) = stride_reg(hexid) {
        return Some(stride_reg);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_korea() {
        assert_eq!(hl_reg(0x71bd54), Some("HL7554".to_string()));
        assert_eq!(hl_reg(0x71c025), Some("HL8025".to_string()));
        // assert_eq!(hl_reg(0x71c523), Some("HL8523".to_string()));
    }

    #[test]
    fn test_japan() {
        assert_eq!(ja_reg(0x869232), Some("JA788A".to_string()));
        assert_eq!(ja_reg(0x86dcc4), Some("JA841J".to_string()));
        assert_eq!(ja_reg(0x847c18), Some("JA19JJ".to_string()));
        assert_eq!(ja_reg(0x3949f9), None);
    }

    #[test]
    fn test_us() {
        assert_eq!(n_reg(0xa43e7f), Some("N37263".to_string()));
        assert_eq!(n_reg(0xa44533), Some("N3741S".to_string()));
        assert_eq!(n_reg(0xad7701), Some("N967JT".to_string()));
    }

    #[test]
    fn test_ra() {
        assert_eq!(numeric_reg(0x140b3a), Some("RA-02874".to_string()));
    }

    #[test]
    fn test_zs() {
        assert_eq!(stride_reg(0x008016), Some("ZS-AAF".to_string()));
        //assert_eq!(stride_reg(0x008071), Some("ZS-AHL".to_string()));
    }

    #[test]
    fn test_f() {
        assert_eq!(stride_reg(0x39b415), Some("F-HNAV".to_string()));
        assert_eq!(stride_reg(0x39cf09), Some("F-HTYJ".to_string()));
        assert_eq!(stride_reg(0x3949f9), Some("F-GSPZ".to_string()));
    }
}
