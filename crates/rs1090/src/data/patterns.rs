use std::num::ParseIntError;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::tail::tail;

#[derive(Debug, Deserialize)]
pub struct Patterns {
    pub registers: Vec<Register>,
    pub properties: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Register {
    pub pattern: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub country: String,
    pub flag: String,
    pub comment: Option<String>,
    pub categories: Option<Vec<Category>>,
}

#[derive(Debug, Deserialize)]
pub struct Category {
    pub pattern: String,
    pub category: Option<String>,
    pub country: Option<String>,
    pub flag: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AircraftInformation {
    pub icao24: String,
    pub registration: Option<String>,
    pub country: Option<String>,
    pub flag: Option<String>,
    pub pattern: Option<String>,
    pub category: Option<String>,
    pub comment: Option<String>,
}

const PATTERNS_JSON: &str = include_str!("../../data/patterns.json");
pub static PATTERNS: Lazy<Patterns> =
    Lazy::new(|| serde_json::from_str(PATTERNS_JSON).unwrap());

pub fn aircraft_information(
    icao24: &str,
    registration: Option<&str>,
) -> Result<AircraftInformation, ParseIntError> {
    let hexid = u32::from_str_radix(icao24, 16)?;

    let mut res = AircraftInformation {
        icao24: icao24.to_ascii_lowercase(),
        ..Default::default()
    };
    res.registration = tail(hexid);
    if let Some(tail) = registration {
        res.registration = Some(tail.to_string());
    }

    if let Some(pattern) = &PATTERNS.registers.iter().find(|elt| {
        if let Some(start) = &elt.start {
            if let Some(end) = &elt.end {
                let start = u32::from_str_radix(&start[2..], 16).unwrap();
                let end = u32::from_str_radix(&end[2..], 16).unwrap();
                return (hexid >= start) & (hexid <= end);
            }
        }
        false
    }) {
        res.country = Some(pattern.country.to_string());
        res.flag = Some(pattern.flag.to_string());
        if let Some(p) = &pattern.pattern {
            res.pattern = Some(p.to_string())
        }
        if let Some(comment) = &pattern.comment {
            res.comment = Some(comment.to_string())
        }

        if let Some(tail) = &res.registration {
            if let Some(categories) = &pattern.categories {
                if let Some(cat) = categories.iter().find(|elt| {
                    let re = Regex::new(&elt.pattern).unwrap();
                    re.is_match(tail)
                }) {
                    res.pattern = Some(cat.pattern.to_string());
                    if let Some(category) = &cat.category {
                        res.category = Some(category.to_string());
                    }
                    if let Some(country) = &cat.country {
                        res.country = Some(country.to_string());
                    }
                    if let Some(flag) = &cat.flag {
                        res.flag = Some(flag.to_string());
                    }
                }
            }
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::PATTERNS;

    #[test]
    fn test_find_country() {
        for register in &PATTERNS.registers {
            if register.country == "France" {
                assert_eq!(register.start, Some("0x380000".to_string()));
                return;
            }
        }
        unreachable!()
    }
}
