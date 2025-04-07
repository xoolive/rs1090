#![allow(clippy::suspicious_else_formatting)]

use deku::prelude::*;
use serde::{Deserialize, ser::SerializeStruct};
use serde::Serialize;
use std::fmt;

/**
 * ## Airborne Velocity (BDS 0,9)
 *
 * Airborne velocities are all transmitted with Type Code 19. Four different
 * subtypes are defined in bits 6-8 of the ME field. All sub-types share a
 * similar overall message structure.
 *
 * Subtypes 1 and 2 are used to report ground speeds of aircraft. Subtypes 3 and
 * 4 are used to report aircraft true airspeed or indicated airspeed. Reporting
 * of airspeed in ADS-B only occurs when aircraft position cannot be determined
 * based on the GNSS system. In the real world, subtype 3 messages are very
 * rare.
 *
 * Subtypes 2 and 4 are designed for supersonic aircraft. Their message
 * structures are identical to subtypes 1 and 3, but with the speed resolution
 * of 4 kt instead of 1 kt. However, since there are no operational supersonic
 * airliners currently, there is no ADS-B airborne velocity message with
 * subtypes 2 and 4 at this moment.
 *
 */
#[derive(Debug, PartialEq, Serialize, Deserialize, DekuRead, Clone)]
pub struct AirborneVelocity {
    #[deku(bits = "3")]
    #[serde(skip)]
    /// The subtype value
    pub subtype: u8,

    #[deku(bits = "1")]
    #[serde(skip)]
    /// The intent change flag
    pub intent_change: bool,

    #[deku(bits = "1")]
    #[serde(skip)]
    /// The IFR capability flag
    pub ifr_capability: bool,

    #[deku(bits = "3")]
    #[serde(rename = "NACv")]
    /// The Navigation Accuracy Category, velocity (NACv)
    ///
    /// It is a NUCv if ADS-B version is 0.
    pub nac_v: u8,

    #[deku(ctx = "*subtype")]
    #[serde(flatten)]
    /// Contains a ground or an air speed depending on the subtype
    pub velocity: AirborneVelocitySubType,

    /// The source for the vertical rate measurement
    pub vrate_src: VerticalRateSource,

    #[serde(skip)]
    /// The sign of the vertical rate value
    pub vrate_sign: Sign,
    #[deku(
        endian = "big",
        bits = "9",
        map = "|v: u16| -> Result<_, DekuError> {
            if v == 0 { Ok(None) }
            else {
                Ok(Some(vrate_sign.value() * (v as i16 - 1)  * 64))
            }
        }"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The vertical rate value in ft/mn, None if unavailable
    pub vertical_rate: Option<i16>,

    #[deku(bits = "2")]
    #[serde(skip)]
    pub reserved: u8,

    #[serde(skip)]
    /// The sign of the difference between the GNSS height and the barometric altitude
    pub gnss_sign: Sign,

    #[deku(reader = "read_geobaro(deku::reader, *gnss_sign)")]
    /// The signed difference between the GNSS height and the barometric altitude
    pub geo_minus_baro: Option<i16>,
}

fn read_geobaro<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
    gnss_sign: Sign,
) -> Result<Option<i16>, DekuError> {
    let value = u8::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(7)),
    )?;
    let value = if value > 1 {
        match gnss_sign {
            Sign::Positive => Some(25 * (value as i16 - 1)),
            Sign::Negative => Some(-25 * (value as i16 - 1)),
        }
    } else {
        None
    };
    Ok(value)
}

#[derive(Debug, PartialEq, Serialize, Deserialize, DekuRead, Clone)]
#[deku(ctx = "subtype: u8", id = "subtype")]
#[serde(untagged)]
pub enum AirborneVelocitySubType {
    #[deku(id = "0")]
    Reserved0(#[deku(bits = "22")] u32),

    #[deku(id_pat = "1..=2")]
    GroundSpeedDecoding(GroundSpeedDecoding),

    #[deku(id = "3")]
    AirspeedSubsonic(AirspeedSubsonicDecoding),
    #[deku(id = "4")]
    AirspeedSupersonic(AirspeedSupersonicDecoding),

    #[deku(id_pat = "5..=7")]
    Reserved1(#[deku(bits = "22")] u32),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "1")]
pub enum Sign {
    #[serde(rename = "positive")]
    Positive = 0,
    #[serde(rename = "negative")]
    Negative = 1,
}

impl Default for Sign {
    fn default() -> Self {
        Self::Positive
    }
}

impl Sign {
    #[must_use]
    pub fn value(&self) -> i16 {
        match self {
            Self::Positive => 1,
            Self::Negative => -1,
        }
    }
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Positive => "",
                Self::Negative => "-",
            }
        )
    }
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct GroundSpeedDecoding {
    #[serde(skip)]
    pub ew_sign: Sign,
    #[deku(
        endian = "big",
        bits = "10",
        map = "|val: u16| -> Result<_, DekuError> {
            Ok(f64::from((val as i16 - 1) * ew_sign.value()))
        }"
    )]
    #[serde(skip)]
    pub ew_vel: f64,
    #[serde(skip)]
    pub ns_sign: Sign,
    #[serde(skip)]
    #[deku(
        endian = "big",
        bits = "10",
        map = "|val: u16| -> Result<_, DekuError> {
            Ok(f64::from((val as i16 - 1) * ns_sign.value()))
        }"
    )]
    pub ns_vel: f64,
    #[deku(
        skip,
        default = "libm::hypot(f64::abs(*ew_vel), f64::abs(*ns_vel))"
    )]
    pub groundspeed: f64,
    #[deku(
        skip,
        default = "
        let h = libm::atan2(*ew_vel, *ns_vel) *
            (360.0 / (2.0 * std::f64::consts::PI));
        if h < 0.0 { h + 360. } else { h }
        "
    )]
    pub track: f64,
}

// Implement Deserialize for GroundSpeedDecoding
impl<'de> Deserialize<'de> for GroundSpeedDecoding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        // Create a simple struct that deserializes the fields we need
        #[derive(Deserialize)]
        struct GroundSpeedHelper {
            groundspeed: f64,
            track: f64,
        }

        let helper = GroundSpeedHelper::deserialize(deserializer)?;

        // Create a default instance but with the values from the deserialization
        Ok(GroundSpeedDecoding {
            ew_sign: Sign::Positive,
            ew_vel: 0.0,
            ns_sign: Sign::Positive,
            ns_vel: 0.0,
            groundspeed: helper.groundspeed,
            track: helper.track,
        })
    }
}

#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct AirspeedSubsonicDecoding {
    #[deku(bits = "1")]
    pub status_heading: bool,

    #[deku(
        endian = "big",
        bits = "10",
        map = "|val: u16| -> Result<_, DekuError> {
            Ok(if *status_heading { Some(val as f64 * 360. / 1024.) } else { None })
        }"
    )]
    pub heading: Option<f64>,

    pub airspeed_type: AirspeedType,

    #[deku(
        endian = "big",
        bits = "10",
        map = "|value: u16| -> Result<_, DekuError> {
            if value == 0 { return Ok(None) }
            Ok(Some(value - 1))
        }"
    )]
    pub airspeed: Option<u16>,
}

impl Serialize for AirspeedSubsonicDecoding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Message", 2)?;
        if let Some(heading) = &self.heading {
            state.serialize_field("heading", heading)?;
        }
        if let Some(airspeed) = &self.airspeed {
            match &self.airspeed_type {
                AirspeedType::IAS => {
                    state.serialize_field("IAS", &airspeed)?;
                }
                AirspeedType::TAS => {
                    state.serialize_field("TAS", &airspeed)?;
                }
            }
        }
        state.end()
    }
}

// Add Deserialize implementation for AirspeedSubsonicDecoding
impl<'de> Deserialize<'de> for AirspeedSubsonicDecoding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        // Create a helper struct that can be properly deserialized
        #[derive(Deserialize)]
        struct Helper {
            heading: Option<f64>,
            #[serde(rename = "IAS")]
            ias: Option<u16>,
            #[serde(rename = "TAS")]
            tas: Option<u16>,
        }

        let helper = Helper::deserialize(deserializer)?;

        // Determine airspeed type and value based on which field is present
        let airspeed_type = if helper.tas.is_some() {
            AirspeedType::TAS
        } else {
            AirspeedType::IAS
        };

        let airspeed = helper.tas.or(helper.ias);

        Ok(Self {
            status_heading: helper.heading.is_some(),
            heading: helper.heading,
            airspeed_type,
            airspeed,
        })
    }
}

#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct AirspeedSupersonicDecoding {
    #[deku(bits = "1")]
    pub status_heading: bool,

    #[deku(
        endian = "big",
        bits = "10",
        map = "|val: u16| -> Result<_, DekuError> {
            Ok(if *status_heading { Some(val as f32 * 360. / 1024.) } else { None })
        }"
    )]
    pub heading: Option<f32>,

    pub airspeed_type: AirspeedType,

    #[deku(
        endian = "big",
        bits = "10",
        map = "|value: u16| -> Result<_, DekuError> {
            if value == 0 { return Ok(None) }
            Ok(Some(4*(value - 1)))
        }"
    )]
    pub airspeed: Option<u16>,
}

impl Serialize for AirspeedSupersonicDecoding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Message", 2)?;
        if let Some(heading) = &self.heading {
            state.serialize_field("heading", heading)?;
        }
        if let Some(airspeed) = &self.airspeed {
            match &self.airspeed_type {
                AirspeedType::IAS => {
                    state.serialize_field("IAS", &airspeed)?;
                }
                AirspeedType::TAS => {
                    state.serialize_field("TAS", &airspeed)?;
                }
            }
        }
        state.end()
    }
}

// Add Deserialize implementation for AirspeedSupersonicDecoding
impl<'de> Deserialize<'de> for AirspeedSupersonicDecoding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        // Create a helper struct that can be properly deserialized
        #[derive(Deserialize)]
        struct Helper {
            heading: Option<f32>,
            #[serde(rename = "IAS")]
            ias: Option<u16>,
            #[serde(rename = "TAS")]
            tas: Option<u16>,
        }

        let helper = Helper::deserialize(deserializer)?;

        // Determine airspeed type and value based on which field is present
        let airspeed_type = if helper.tas.is_some() {
            AirspeedType::TAS
        } else {
            AirspeedType::IAS
        };

        let airspeed = helper.tas.or(helper.ias);

        Ok(Self {
            status_heading: helper.heading.is_some(),
            heading: helper.heading,
            airspeed_type,
            airspeed,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead, Deserialize)]
#[deku(id_type = "u8", bits = "1")]
pub enum AirspeedType {
    IAS = 0,
    TAS = 1,
}

impl fmt::Display for AirspeedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::IAS => "IAS",
                Self::TAS => "TAS",
            }
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
#[deku(id_type = "u8", bits = "1")]
pub enum DirectionEW {
    WestToEast = 0,
    EastToWest = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
#[deku(id_type = "u8", bits = "1")]
pub enum DirectionNS {
    SouthToNorth = 0,
    NorthToSouth = 1,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "1")]
pub enum VerticalRateSource {
    #[serde(rename = "barometric")]
    BarometricPressureAltitude = 0,

    #[serde(rename = "GNSS")]
    GeometricAltitude = 1,
}

impl Default for VerticalRateSource {
    fn default() -> Self {
        Self::BarometricPressureAltitude
    }
}

impl fmt::Display for VerticalRateSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::BarometricPressureAltitude => "barometric",
                Self::GeometricAltitude => "GNSS",
            }
        )
    }
}

impl fmt::Display for AirborneVelocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Airborne velocity over ground (BDS 0,9)")?;
        match &self.velocity {
            AirborneVelocitySubType::GroundSpeedDecoding(v) => {
                writeln!(f, "  Track angle:   {}°", libm::round(v.track))?;
                writeln!(
                    f,
                    "  Groundspeed:   {} kt",
                    libm::round(v.groundspeed)
                )?;
            }
            AirborneVelocitySubType::AirspeedSubsonic(v) => {
                if let Some(value) = v.airspeed {
                    writeln!(
                        f,
                        "  {}:           {} kt",
                        v.airspeed_type, value
                    )?;
                }
                if let Some(value) = v.heading {
                    writeln!(f, "  Heading:       {}°", libm::round(value))?;
                }
            }
            AirborneVelocitySubType::AirspeedSupersonic(v) => {
                if let Some(value) = v.airspeed {
                    writeln!(
                        f,
                        "  {}:           {} kt",
                        v.airspeed_type, value
                    )?;
                    if let Some(value) = v.heading {
                        writeln!(
                            f,
                            "  Heading:       {}°",
                            libm::round(value as f64)
                        )?;
                    }
                }
            }
            AirborneVelocitySubType::Reserved0(_)
            | AirborneVelocitySubType::Reserved1(_) => {}
        }
        if let Some(vr) = &self.vertical_rate {
            writeln!(f, "  Vertical rate: {} ft/min {}", vr, &self.vrate_src)?;
        }
        writeln!(f, "  NACv:          {}", &self.nac_v)?;
        if let Some(value) = &self.geo_minus_baro {
            writeln!(f, "  GNSS delta:    {} ft", value)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_groundspeed_velocity() {
        let bytes = hex!("8D485020994409940838175B284F");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            if let ME::BDS09(velocity) = adsb_msg.message {
                if let AirborneVelocitySubType::GroundSpeedDecoding(_gsd) =
                    velocity.velocity
                {
                    assert_relative_eq!(
                        _gsd.groundspeed,
                        159.,
                        max_relative = 1e-2
                    );
                    assert_relative_eq!(
                        _gsd.track,
                        182.88,
                        max_relative = 1e-2
                    );
                    if let Some(vrate) = velocity.vertical_rate {
                        assert_eq!(vrate, -832);
                    }
                    assert_eq!(velocity.geo_minus_baro, Some(550));
                }
                return;
            }
        }
        unreachable!();
    }

    #[test]
    fn test_format_groundspeed() {
        let bytes = hex!("8D485020994409940838175B284F");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        assert_eq!(
            format!("{msg}"),
            r#" DF17. Extended Squitter
  Address:       485020
  Air/Ground:    airborne
  Airborne velocity over ground (BDS 0,9)
  Track angle:   183°
  Groundspeed:   159 kt
  Vertical rate: -832 ft/min barometric
  NACv:          0
  GNSS delta:    550 ft
"#
        )
    }

    #[test]
    fn test_airspeed_velocity() {
        let bytes = hex!("8DA05F219B06B6AF189400CBC33F");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            if let ME::BDS09(velocity) = adsb_msg.message {
                if let AirborneVelocitySubType::AirspeedSubsonic(asd) =
                    velocity.velocity
                {
                    assert_eq!(asd.airspeed.unwrap(), 375);
                    assert_relative_eq!(
                        asd.heading.unwrap(),
                        244.,
                        max_relative = 1e-2
                    );
                    assert_eq!(velocity.vertical_rate.unwrap(), -2304);
                }
                return;
            }
        }
        unreachable!();
    }

    #[test]
    fn test_format_airspeed() {
        let bytes = hex!("8DA05F219B06B6AF189400CBC33F");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        assert_eq!(
            format!("{msg}"),
            r#" DF17. Extended Squitter
  Address:       a05f21
  Air/Ground:    airborne
  Airborne velocity over ground (BDS 0,9)
  TAS:           375 kt
  Heading:       244°
  Vertical rate: -2304 ft/min GNSS
  NACv:          0
"#
        )
    }

    #[test]
    fn test_airborne_velocity_serde_json() {
        use serde_json;

        let bytes = hex!("8D485020994409940838175B284F");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            if let ME::BDS09(velocity) = adsb_msg.message {
                // Serialize to JSON
                let json = serde_json::to_string(&velocity).unwrap();

                // Deserialize back
                let deserialized: AirborneVelocity = serde_json::from_str(&json).unwrap();

                // Check basic equality
                assert_eq!(velocity.nac_v, deserialized.nac_v);
                assert_eq!(velocity.vertical_rate, deserialized.vertical_rate);

                // Check specific velocity subtype fields
                match (&velocity.velocity, &deserialized.velocity) {
                    (AirborneVelocitySubType::GroundSpeedDecoding(v1),
                     AirborneVelocitySubType::GroundSpeedDecoding(v2)) => {
                        assert_relative_eq!(v1.groundspeed, v2.groundspeed, max_relative = 1e-3);
                        assert_relative_eq!(v1.track, v2.track, max_relative = 1e-3);
                    },
                    _ => panic!("Expected GroundSpeedDecoding"),
                }
                return;
            }
        }
        panic!("Expected AirborneVelocity message");
    }

    #[test]
    fn test_airborne_velocity_serde_msgpack() {
        use rmp_serde::{Deserializer, Serializer};
        use serde::{Deserialize, Serialize};
        use std::io::Cursor;

        let bytes = hex!("8D485020994409940838175B284F");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let ExtendedSquitterADSB(adsb_msg) = msg.df {
            if let ME::BDS09(velocity) = adsb_msg.message {
                // Serialize to MessagePack
                let mut buf = Vec::new();
                velocity.serialize(&mut Serializer::new(&mut buf)).unwrap();

                // Deserialize back
                let mut de = Deserializer::new(Cursor::new(&buf));
                let deserialized: AirborneVelocity = Deserialize::deserialize(&mut de).unwrap();

                // Check basic equality
                assert_eq!(velocity.nac_v, deserialized.nac_v);
                assert_eq!(velocity.vertical_rate, deserialized.vertical_rate);

                // Check specific velocity subtype fields
                match (&velocity.velocity, &deserialized.velocity) {
                    (AirborneVelocitySubType::GroundSpeedDecoding(v1),
                     AirborneVelocitySubType::GroundSpeedDecoding(v2)) => {
                        assert_relative_eq!(v1.groundspeed, v2.groundspeed, max_relative = 1e-3);
                        assert_relative_eq!(v1.track, v2.track, max_relative = 1e-3);
                    },
                    _ => panic!("Expected GroundSpeedDecoding"),
                }
                return;
            }
        }
        panic!("Expected AirborneVelocity message");
    }
}
