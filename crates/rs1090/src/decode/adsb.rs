use super::bds::{bds05, bds06, bds08, bds09, bds61, bds62, bds65};
use super::{Capability, ICAO};
use deku::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/**
 * An ADS-B frame is 112 bits long.
 *
 * It consists of five main parts, shown as follows:
 *
 * | DF  | CA  | ICAO | ME  | PI  |
 * | --- | --- | ---- | --- | --- |
 * | 5   | 3   | 24   | 56  | 24  |
 *
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct ADSB {
    /// The transponder capability
    #[serde(rename = "capability")]
    pub capability: Capability,

    /// The ICAO aircraft address on 24 bytes
    pub icao24: ICAO,

    /// The message, prefixed by a typecode on 5 bytes
    #[serde(flatten)]
    pub message: ME,

    /// Parity/Interrogator ID
    // Change from skip_serializing to include in serialization
    pub parity: ICAO,
}

// Modify the custom Deserialize implementation to properly handle the parity field
impl<'de> Deserialize<'de> for ADSB {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ADSBHelper {
            #[serde(rename = "capability")]
            capability: Capability,
            icao24: ICAO,
            #[serde(flatten)]
            message: ME,
            // Make the parity optional with a default value
            #[serde(default)]
            parity: ICAO,
        }

        let helper = ADSBHelper::deserialize(deserializer)?;

        Ok(ADSB {
            capability: helper.capability,
            icao24: helper.icao24,
            message: helper.message,
            parity: helper.parity,
        })
    }
}

/**
* The first 5 bytes of the Message Field [`ME`] encode the typecode.
*
* It is used to identify which kind of data is encode in the following bytes.
*
* | Typecode | Name                                              |
* | -------- | ------------------------------------------------- |
* | 0        | [`ME::NoPosition`]                                |
* | 1..=4    | [`bds08::AircraftIdentification`]                 |
* | 5..=8    | [`bds06::SurfacePosition`]                        |
* | 9..=18   | [`bds05::AirbornePosition`] (barometric altitude) |
* | 19       | [`bds09::AirborneVelocity`]                       |
* | 20..=22  | [`bds05::AirbornePosition`] (GNSS height)         |
* | 23       | [`ME::Reserved0`]                                 |
* | 24       | [`ME::SurfaceSystemStatus`]                       |
* | 25..=27  | [`ME::Reserved1`]                                 |
* | 28       | [`bds61::AircraftStatus`]                         |
* | 29       | [`bds62::TargetStateAndStatusInformation`]        |
* | 30       | [`ME::AircraftOperationalCoordination`]           |
* | 31       | [`bds65::AircraftOperationStatus`]                |
*/

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct Unused {
    #[deku(skip, pad_bits_after = "48", default = "true")]
    #[serde(skip)]
    pub unused: bool,
}

// Custom Deserialize implementation for Unused
impl<'de> Deserialize<'de> for Unused {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        // Create a struct that just deserializes nothing and returns
        // the Unused struct with the correct default value
        struct UnusedVisitor;

        impl<'de> serde::de::Visitor<'de> for UnusedVisitor {
            type Value = Unused;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an empty struct")
            }

            fn visit_map<A>(self, _map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                // Always return with unused set to true
                Ok(Unused { unused: true })
            }
        }

        // Use an empty structure deserializer
        deserializer.deserialize_map(UnusedVisitor)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, DekuRead, Clone)]
#[deku(id_type = "u8", bits = "5")]
#[serde(tag = "bds")]
pub enum ME {
    #[deku(id = "0")]
    NoPosition(Unused),

    #[deku(id_pat = "1..=4")]
    #[serde(rename = "08")]
    BDS08(bds08::AircraftIdentification),

    #[deku(id_pat = "5..=8")]
    #[serde(rename = "06")]
    BDS06(bds06::SurfacePosition),

    #[deku(id_pat = "9..=18 | 20..=22")]
    #[serde(rename = "05")]
    BDS05(bds05::AirbornePosition),

    #[deku(id = "19")]
    #[serde(rename = "09")]
    BDS09(bds09::AirborneVelocity),

    #[deku(id = "23")]
    #[serde(rename = "id23")]
    Reserved0(Unused),

    #[deku(id = "24")]
    #[serde(rename = "id24")]
    SurfaceSystemStatus(Unused),

    #[deku(id_pat = "25..=27")]
    #[serde(rename = "id25_27")]
    Reserved1 { unused: u8 },

    #[deku(id = "28")]
    #[serde(rename = "61")]
    BDS61(bds61::AircraftStatus),

    #[deku(id = "29")]
    #[serde(rename = "62")]
    BDS62(bds62::TargetStateAndStatusInformation),

    #[deku(id = "30")]
    #[serde(rename = "id30")]
    AircraftOperationalCoordination(Unused),

    #[deku(id = "31")]
    #[serde(rename = "65")]
    BDS65(bds65::AircraftOperationStatus),
}

impl fmt::Display for ME {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ME::NoPosition { .. }
            | ME::Reserved0 { .. }
            | ME::Reserved1 { .. }
            | ME::SurfaceSystemStatus { .. }
            | ME::AircraftOperationalCoordination { .. } => Ok(()),
            ME::BDS05(me) => {
                write!(f, "{}", me)
            }
            ME::BDS06(me) => {
                write!(f, "{}", me)
            }
            ME::BDS08(me) => {
                write!(f, "{}", me)
            }
            ME::BDS09(me) => {
                write!(f, "{}", me)
            }
            ME::BDS61(me) => {
                write!(f, "{}", me)
            }
            ME::BDS62(me) => {
                write!(f, "{}", me)
            }
            ME::BDS65(me) => {
                write!(f, "{}", me)
            }
        }
    }
}

impl fmt::Display for ADSB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, " DF17. Extended Squitter")?;
        writeln!(f, "  Address:       {}", &self.icao24)?;
        writeln!(f, "  Air/Ground:    {}", &self.capability)?;
        write!(f, "{}", &self.message)
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_icao24() {
        let bytes = hex!("8D406B902015A678D4D220AA4BDA");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(msg) = msg.df {
            assert_eq!(format!("{}", msg.icao24), "406b90");
            return;
        }
        unreachable!();
    }

    #[test]
    fn test_adsb_serde_json() {
        use serde_json;

        let bytes = hex!("8D406B902015A678D4D220AA4BDA");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            // Serialize to JSON
            let json = serde_json::to_string(&adsb_msg).unwrap();

            // Deserialize back
            let deserialized: ADSB = serde_json::from_str(&json).unwrap();

            // Check equality
            assert_eq!(adsb_msg.icao24, deserialized.icao24);
            assert_eq!(adsb_msg.capability, deserialized.capability);

            match (&adsb_msg.message, &deserialized.message) {
                (ME::BDS08(orig), ME::BDS08(deser)) => {
                    assert_eq!(orig.callsign, deser.callsign);
                },
                _ => panic!("Expected BDS08 message type"),
            }
        } else {
            panic!("Expected ADSB message");
        }
    }

    #[test]
    fn test_adsb_serde_msgpack() {
        use rmp_serde::{Deserializer, Serializer};
        use serde::{Deserialize, Serialize};
        use std::io::Cursor;

        let bytes = hex!("8D406B902015A678D4D220AA4BDA");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            // Serialize to MessagePack
            let mut buf = Vec::new();
            adsb_msg.serialize(&mut Serializer::new(&mut buf)).unwrap();

            // Deserialize back
            let mut de = Deserializer::new(Cursor::new(&buf));
            let deserialized: ADSB = Deserialize::deserialize(&mut de).unwrap();

            // Check equality
            assert_eq!(adsb_msg.icao24, deserialized.icao24);
            assert_eq!(adsb_msg.capability, deserialized.capability);

            match (&adsb_msg.message, &deserialized.message) {
                (ME::BDS08(orig), ME::BDS08(deser)) => {
                    assert_eq!(orig.callsign, deser.callsign);
                },
                _ => panic!("Expected BDS08 message type"),
            }
        } else {
            panic!("Expected ADSB message");
        }
    }
}
