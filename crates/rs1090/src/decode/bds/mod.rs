pub mod bds05;
pub mod bds06;
pub mod bds08;
pub mod bds09;
pub mod bds10;
pub mod bds17;
pub mod bds20;
pub mod bds61;
pub mod bds62;
pub mod bds65;

use serde::ser::Serializer;

/*
// TODO change this to a better one
#[derive(Debug, PartialEq, Eq, DekuRead, Clone)]
#[deku(type = "u8", bits = "8")]
pub enum BDS {
    /// (1, 0) Table A-2-16
    #[deku(id = "0x00")]
    Empty([u8; 6]),

    /// (1, 0) Table A-2-16
    #[deku(id = "0x10")]
    DataLinkCapability(bds10::DataLinkCapability),

    /// (2, 0) Table A-2-32
    #[deku(id = "0x20")]
    AircraftIdentification(
        #[deku(reader = "bds08::callsign_read(deku::rest)")] String,
    ),

    #[deku(id_pat = "_")]
    Unknown([u8; 6]),
}

*/

fn f64_twodecimals<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded_value = (value * 100.0).round() / 100.0; // Round to two decimals
    serializer.serialize_f64(rounded_value)
}
