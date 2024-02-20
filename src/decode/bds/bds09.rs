extern crate alloc;

use alloc::fmt;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct AirborneVelocity {
    #[deku(bits = "3")]
    pub subtype: u8,
    #[deku(bits = "5")]
    pub nac_v: u8,
    #[deku(ctx = "*subtype")]
    pub sub_type: AirborneVelocitySubType,
    pub vrate_src: VerticalRateSource,
    pub vrate_sign: Sign,
    #[deku(endian = "big", bits = "9")]
    pub vrate_value: u16,
    #[deku(bits = "2")]
    pub reverved: u8,
    pub gnss_sign: Sign,
    #[deku(
        bits = "7",
        map = "|gnss_baro_diff: u16| -> Result<_, DekuError> {Ok(if gnss_baro_diff > 1 {(gnss_baro_diff - 1) * 25} else { 0 })}"
    )]
    pub gnss_baro_diff: u16,
}

impl AirborneVelocity {
    /// Return effective (`track`, `ground_speed`, `vertical_rate`) for groundspeed
    #[must_use]
    pub fn calculate(&self) -> Option<(f32, f64, i16)> {
        if let AirborneVelocitySubType::GroundSpeedDecoding(ground_speed) = &self.sub_type {
            let v_ew = f64::from((ground_speed.ew_vel as i16 - 1) * ground_speed.ew_sign.value());
            let v_ns = f64::from((ground_speed.ns_vel as i16 - 1) * ground_speed.ns_sign.value());
            let h = libm::atan2(v_ew, v_ns) * (360.0 / (2.0 * std::f64::consts::PI));
            let track = if h < 0.0 { h + 360.0 } else { h };
            let speed = libm::hypot(v_ew, v_ns);
            let vrate = self
                .vrate_value
                .checked_sub(1)
                .and_then(|v| v.checked_mul(64))
                .map(|v| (v as i16) * self.vrate_sign.value());

            if let Some(vrate) = vrate {
                return Some((track as f32, speed, vrate));
            }
        }
        None
    }
}

/// Airborne Velocity Message “Subtype” Code Field Encoding
#[derive(Debug, PartialEq, DekuRead, Clone)]
#[deku(ctx = "subtype: u8", id = "subtype")]
pub enum AirborneVelocitySubType {
    #[deku(id = "0")]
    Reserved0(#[deku(bits = "22")] u32),

    #[deku(id_pat = "1..=2")]
    GroundSpeedDecoding(GroundSpeedDecoding),

    #[deku(id_pat = "3..=4")]
    AirspeedDecoding(AirspeedDecoding),

    #[deku(id_pat = "5..=7")]
    Reserved1(#[deku(bits = "22")] u32),
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum Sign {
    Positive = 0,
    Negative = 1,
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

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
pub struct GroundSpeedDecoding {
    pub ew_sign: Sign,
    #[deku(endian = "big", bits = "10")]
    pub ew_vel: u16,
    pub ns_sign: Sign,
    #[deku(endian = "big", bits = "10")]
    pub ns_vel: u16,
}

#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct AirspeedDecoding {
    #[deku(bits = "1")]
    pub status_heading: u8,
    #[deku(
        endian = "big",
        bits = "10",
        map = "|mag_heading: u16| -> Result<_, DekuError> { Ok(mag_heading as f32 * 360. / 1024.)}"
    )]
    pub mag_heading: f32,
    pub airspeed_type: AirspeedType,
    #[deku(
        endian = "big",
        bits = "10",
        map = "|airspeed: u16| -> Result<_, DekuError> { Ok(if airspeed > 0 { airspeed - 1 } else { 0 })}"
    )]
    pub airspeed: u16,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead)]
#[deku(type = "u8", bits = "1")]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead)]
#[deku(type = "u8", bits = "3")]
pub enum AirborneVelocityType {
    Subsonic = 1,
    Supersonic = 3,
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(ctx = "subtype: AirborneVelocityType")]
pub struct AirborneVelocitySubFields {
    pub dew: DirectionEW,
    #[deku(reader = "Self::read_v(deku::rest, subtype)")]
    pub vew: u16,
    pub dns: DirectionNS,
    #[deku(reader = "Self::read_v(deku::rest, subtype)")]
    pub vns: u16,
}

impl AirborneVelocitySubFields {
    fn read_v(
        rest: &BitSlice<u8, Msb0>,
        subtype: AirborneVelocityType,
    ) -> Result<(&BitSlice<u8, Msb0>, u16), DekuError> {
        match subtype {
            AirborneVelocityType::Subsonic => {
                u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))
                    .map(|(rest, value)| (rest, value - 1))
            }
            AirborneVelocityType::Supersonic => {
                u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(10)))
                    .map(|(rest, value)| (rest, 4 * (value - 1)))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead)]
#[deku(type = "u8", bits = "1")]
pub enum DirectionEW {
    WestToEast = 0,
    EastToWest = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead)]
#[deku(type = "u8", bits = "1")]
pub enum DirectionNS {
    SouthToNorth = 0,
    NorthToSouth = 1,
}

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum VerticalRateSource {
    BarometricPressureAltitude = 0,
    GeometricAltitude = 1,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Typecode::AirborneVelocity;
    use crate::decode::{Message, DF::ADSB};
    use hexlit::hex;

    extern crate approx;
    use approx::assert_relative_eq;

    #[test]
    fn test_groundspeed_velocity() {
        let bytes = hex!("8D485020994409940838175B284F");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ADSB(adsb_msg) = msg.df {
            if let AirborneVelocity(velocity) = adsb_msg.message {
                if let AirborneVelocitySubType::GroundSpeedDecoding(_gsd) = velocity.sub_type {
                    if let Some((trk, gs, vr)) = velocity.calculate() {
                        assert_relative_eq!(gs, 159., max_relative = 1e-2);
                        assert_relative_eq!(trk, 182.88, max_relative = 1e-2);
                        assert_eq!(vr, -832);
                        assert_eq!(velocity.gnss_baro_diff, 550);
                    }
                }
                return;
            }
        }
        unreachable!();
    }

    #[test]
    fn test_airspeed_velocity() {
        let bytes = hex!("8DA05F219B06B6AF189400CBC33F");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        if let ADSB(adsb_msg) = msg.df {
            if let AirborneVelocity(velocity) = adsb_msg.message {
                if let AirborneVelocitySubType::AirspeedDecoding(asd) = velocity.sub_type {
                    assert_eq!(asd.airspeed, 375);
                    assert_relative_eq!(asd.mag_heading, 244., max_relative = 1e-2);
                    //assert_eq!(velocity.vrate, -2304);
                }
                return;
            }
        }
        unreachable!();
    }
}
