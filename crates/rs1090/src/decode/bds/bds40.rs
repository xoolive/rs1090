use deku::prelude::*;
use serde::Serialize;

/**
 * ## Selected vertical intention (BDS 4,0)
 */
#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct SelectedVerticalIntention {}
