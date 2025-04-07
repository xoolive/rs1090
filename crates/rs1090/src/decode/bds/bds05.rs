use crate::decode::cpr::CPRFormat;
use crate::decode::{decode_id13, gray2alt};
use deku::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/**
 * ## Airborne Position (BDS 0,5)
 *
 * with barometric altitude (TC=9..=18) or geometric height (TC=20..=22)
 *
 * | TC | SS | SAF | ALT | T | F | LAT-CPR | LON-CPR |
 * | -- | -- | --- | --- | - | - | ------- | ------- |
 * | 5  | 2  |  1  | 12  | 1 | 1 |   17    |   17    |
 */

#[derive(Debug, PartialEq, Serialize, Deserialize, DekuRead, Copy, Clone)]
pub struct AirbornePosition {
    #[deku(bits = 5)]
    tc: u8,

    #[deku(
        skip,
        default = "
        match *tc {
            n if n < 19 => 18 - tc,
            20 | 21 => 29 - tc,
            _ => 0
        }
        "
    )]
    #[serde(rename = "NUCp")]
    /// The Navigation Uncertainty Category Position (NUCp)
    /// (directly based on the typecode)
    pub nuc_p: u8,

    #[serde(skip)]
    /// Decode the surveillance status
    pub ss: SurveillanceStatus,

    #[deku(
        bits = "1",
        map = "|v| -> Result<_, DekuError> {
            if *tc < 19 { Ok(Some(v)) } else { Ok(None) }
        }"
    )]
    #[serde(rename = "NICb", skip_serializing_if = "Option::is_none")]
    /// Single Antenna Flag in ADSB v0 or v1,
    /// Navigation Integrity Category Supplement-b (NICb) in ADSB v2
    pub saf_or_nicb: Option<u8>,

    #[deku(reader = "decode_ac12(deku::reader)")]
    #[serde(rename = "altitude")]
    /// Decode the altitude in feet, encoded on 12 bits.
    /// None if not available.
    pub alt: Option<u16>,

    #[deku(reader = "read_source(*tc)")]
    /// Decode the altitude source (GNSS or barometric),
    /// most commonly equal to barometric
    pub source: Source,

    #[deku(bits = "1")]
    #[serde(skip)]
    // UTC sync or not
    pub t: bool,

    pub parity: CPRFormat,

    #[deku(bits = "17", endian = "big")]
    pub lat_cpr: u32,

    #[deku(bits = "17", endian = "big")]
    pub lon_cpr: u32,

    #[deku(skip, default = "None")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,

    #[deku(skip, default = "None")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

/// Decode altitude value encoded on 12 bits
fn decode_ac12<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<u16>, DekuError> {
    let num = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(12)),
    )?;

    let q = num & 0x10;

    if q > 0 {
        let n = ((num & 0x0fe0) >> 1) | (num & 0x000f);
        let n = n * 25;
        if n > 1000 {
            Ok(Some(n - 1000))
        } else {
            Ok(None)
        }
    } else {
        let mut n = ((num & 0x0fc0) << 1) | (num & 0x003f);
        n = decode_id13(n);
        if let Ok(n) = gray2alt(n) {
            Ok(u16::try_from(n * 100).ok())
        } else {
            Ok(None)
        }
    }
}

fn read_source(tc: u8) -> Result<Source, DekuError> {
    let source = if tc < 19 {
        Source::Barometric
    } else {
        Source::Gnss
    };
    Ok(source)
}

impl fmt::Display for AirbornePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  AirbornePosition (BDS 0,5)")?;
        let altitude = self.alt.map_or_else(
            || "None".to_string(),
            |altitude| format!("{altitude} ft"),
        );
        writeln!(f, "  Altitude:      {} {}", altitude, self.source)?;
        writeln!(f, "  CPR type:      Airborne")?;
        writeln!(f, "  CPR parity:    {}", self.parity)?;
        writeln!(f, "  CPR latitude:  ({})", self.lat_cpr)?;
        writeln!(f, "  CPR longitude: ({})", self.lon_cpr)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "2")]
pub enum SurveillanceStatus {
    #[serde(rename = "no_condition")]
    NoCondition = 0,
    #[serde(rename = "permanent_alert")]
    PermanentAlert = 1,
    #[serde(rename = "temporary_alert")]
    TemporaryAlert = 2,
    #[serde(rename = "spi_condition")]
    SPICondition = 3,
}

impl Default for SurveillanceStatus {
    fn default() -> Self {
        Self::NoCondition
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone)]
pub enum Source {
    #[serde(rename = "barometric")]
    Barometric = 0,
    #[serde(rename = "GNSS")]
    Gnss = 1,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Barometric => "barometric",
                Self::Gnss => "GNSS",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;
    use serde_json;
    use rmp_serde::{Deserializer, Serializer};
    use serde::{Deserialize, Serialize};
    use std::io::Cursor;

    #[test]
    fn test_airborne_position_serde_json() {
        let bytes = hex!("8D40058B58C901375147EFD09357");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();

        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            if let ME::BDS05(ap) = adsb_msg.message {
                // Create a copy with simplified fields for serialization testing
                let test_ap = AirbornePosition {
                    tc: ap.tc,
                    nuc_p: ap.nuc_p,
                    ss: ap.ss,
                    saf_or_nicb: ap.saf_or_nicb,
                    alt: ap.alt,
                    source: ap.source,
                    t: ap.t,
                    parity: ap.parity,
                    lat_cpr: ap.lat_cpr,
                    lon_cpr: ap.lon_cpr,
                    latitude: None,
                    longitude: None,
                };

                // Serialize to JSON
                let json = serde_json::to_string(&test_ap).unwrap();

                // Deserialize back
                let deserialized: AirbornePosition = serde_json::from_str(&json).unwrap();

                // Check equality
                assert_eq!(test_ap.lat_cpr, deserialized.lat_cpr);
                assert_eq!(test_ap.lon_cpr, deserialized.lon_cpr);
                assert_eq!(test_ap.alt, deserialized.alt);
                assert_eq!(test_ap.parity, deserialized.parity);
                return;
            }
        }
        panic!("Expected AirbornePosition message");
    }

    #[test]
    fn test_airborne_position_serde_msgpack() {
        // Create a minimal AirbornePosition object with only the required fields
        let test_position = AirbornePosition {
            tc: 11,
            nuc_p: 7,
            ss: SurveillanceStatus::NoCondition,
            saf_or_nicb: Some(0),
            alt: Some(10000),
            source: Source::Barometric,
            t: false,
            parity: CPRFormat::Even,
            lat_cpr: 92345,
            lon_cpr: 47890,
            latitude: None,
            longitude: None,
        };

        // Use direct serialization methods - no intermediate data structures
        let encoded = rmp_serde::to_vec_named(&test_position).unwrap();

        // Deserialize directly into a new struct
        let deserialized: AirbornePosition = rmp_serde::from_slice(&encoded).unwrap();

        // Check equality of important fields
        assert_eq!(test_position.tc, deserialized.tc);
        assert_eq!(test_position.nuc_p, deserialized.nuc_p);
        assert_eq!(test_position.alt, deserialized.alt);
        assert_eq!(test_position.lat_cpr, deserialized.lat_cpr);
        assert_eq!(test_position.lon_cpr, deserialized.lon_cpr);
        assert_eq!(test_position.parity, deserialized.parity);
    }
}
