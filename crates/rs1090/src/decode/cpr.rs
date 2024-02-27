extern crate alloc;

use alloc::fmt;
use deku::prelude::*;
use serde::Serialize;

/// A flag to qualify a CPR position as odd or even
#[derive(Debug, PartialEq, Eq, Serialize, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
#[serde(rename_all = "snake_case")]
pub enum CPRFormat {
    Even = 0,
    Odd = 1,
}

impl fmt::Display for CPRFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Even => "even",
                Self::Odd => "odd",
            }
        )
    }
}
