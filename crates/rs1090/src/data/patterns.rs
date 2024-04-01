use once_cell::sync::Lazy;
use serde::Deserialize;

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

const PATTERNS_JSON: &str = include_str!("../../data/patterns.json");
pub static PATTERNS: Lazy<Patterns> =
    Lazy::new(|| serde_json::from_str(PATTERNS_JSON).unwrap());

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
