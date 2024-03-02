pub mod adsb;
pub mod bds;
pub mod commb;
pub mod cpr;
pub mod crc;

use adsb::{ADSB, ME};
use commb::DataSelector;
use crc::modes_checksum;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::ser::{Serialize, Serializer};
use std::fmt;

/**
 * DF stands for Downlink Format.
 *
 * A number between 0 and 24 encoding the type of the message, and whether it is
 * short (56 bits) or long (112 bits).
 *
 * |  [`DF`]  |  Name                               |  Section    |
 * | -------- | ----------------------------------- | ----------- |
 * | 0        | [`DF::ShortAirAirSurveillance`]     | 3.1.2.8.2   |
 * | 4        | [`DF::SurveillanceAltitudeReply`]   | 3.1.2.6.5   |
 * | 5        | [`DF::SurveillanceIdentityReply`]   | 3.1.2.6.7   |
 * | 11       | [`DF::AllCallReply`]                | 2.1.2.5.2.2 |
 * | 16       | [`DF::LongAirAirSurveillance`]      | 3.1.2.8.3   |
 * | 17       | [`DF::ExtendedSquitterADSB`]        | 3.1.2.8.6   |
 * | 18       | [`DF::ExtendedSquitterTisB`]        | 3.1.2.8.7   |
 * | 19       | [`DF::ExtendedSquitterMilitary`]    | 3.1.2.8.8   |
 * | 20       | [`DF::CommBAltitudeReply`]          | 3.1.2.6.6   |
 * | 21       | [`DF::CommBIdentityReply`]          | 3.1.2.6.8   |
 * | 24       | [`DF::CommDExtended`]               | 3.1.2.7.3   |
 */

#[derive(Debug, PartialEq, serde::Serialize, DekuRead, Clone)]
#[deku(type = "u8", bits = "5", ctx = "crc: u32")]
#[serde(tag = "DF")]
pub enum DF {
    /// DF=0: Short Air-Air Surveillance (3.1.2.8.2)
    #[deku(id = "0")]
    #[serde(rename = "DF0")]
    ShortAirAirSurveillance {
        /// Vertical status
        #[deku(bits = "1")]
        #[serde(skip)]
        vs: u8,
        /// CC:
        #[deku(bits = "1")]
        #[serde(skip)]
        cc: u8,
        /// unused
        #[deku(bits = "1")]
        #[serde(skip)]
        unused: u8,
        /// Sensitivity level, ACAS
        #[deku(bits = "3")]
        #[serde(skip)]
        sl: u8,
        /// Spare
        #[deku(bits = "2")]
        #[serde(skip)]
        unused1: u8,
        /// Reply information
        #[deku(bits = "4")]
        #[serde(skip)]
        ri: u8,
        /// unused
        #[deku(bits = "2")]
        #[serde(skip)]
        unused2: u8,
        /// Altitude code on 13 bits
        #[serde(rename = "altitude")]
        ac: AC13Field,
        /// ICAO address, parity
        #[serde(rename = "icao24")]
        #[deku(ctx = "crc")]
        ap: IcaoParity,
    },

    /// DF=4: Surveillance Altitude Reply (3.1.2.6.5)
    #[deku(id = "4")]
    #[serde(rename = "DF4")]
    SurveillanceAltitudeReply {
        /// Flight Status
        #[serde(skip)]
        fs: FlightStatus,
        /// DownlinkRequest
        #[serde(skip)]
        dr: DownlinkRequest,
        /// Utility Message
        #[serde(skip)]
        um: UtilityMessage,
        /// Altitude code on 13 bits
        #[serde(rename = "altitude")]
        ac: AC13Field,
        /// Address/Parity
        #[serde(rename = "icao24")]
        #[deku(ctx = "crc")]
        ap: IcaoParity,
    },

    /// DF=5: Surveillance Identity Reply (3.1.2.6.7)
    #[deku(id = "5")]
    #[serde(rename = "DF5")]
    SurveillanceIdentityReply {
        /// Flight Status
        fs: FlightStatus,
        /// Downlink Request
        #[serde(skip)]
        dr: DownlinkRequest,
        /// UtilityMessage
        #[serde(skip)]
        um: UtilityMessage,
        /// Identity code (squawk)
        #[serde(rename = "squawk")]
        id: IdentityCode,
        /// Address/Parity
        #[serde(rename = "icao24")]
        #[deku(ctx = "crc")]
        ap: IcaoParity,
    },

    /// DF=11: (Mode S) All-call reply, Downlink format 11 (2.1.2.5.2.2)
    #[deku(id = "11")]
    #[serde(rename = "DF11")]
    AllCallReply {
        /// Capability
        capability: Capability,
        /// Address Announced
        #[serde(rename = "icao24")]
        icao: ICAO,
        /// Parity/Interrogator identifier
        #[serde(skip)]
        p_icao: ICAO,
    },

    #[deku(id = "16")]
    #[serde(rename = "DF16")]
    /// Long Air-Air Surveillance, Downlink Format 16 (3.1.2.8.3)
    LongAirAirSurveillance {
        #[deku(bits = "1")]
        /// Vertical Status (airborne: 0, onground: 1)
        vs: u8,
        #[deku(bits = "2")]
        #[serde(skip)]
        reserved1: u8,
        #[deku(bits = "3")]
        /// Sensitivity Level (inoperative: 0)
        sl: u8,
        #[deku(bits = "2")]
        #[serde(skip)]
        reserved2: u8,
        #[deku(bits = "4")]
        /// Reply information
        ///
        /// - 0000: No operating ACAS
        /// - 0010: ACAS with resolution capability inhibited
        /// - 0011: ACAS with vertical-only resolution capability
        /// - 0111: ACAS with vertical and horizontal resolution capability
        ri: u8,
        #[deku(bits = "2")]
        #[serde(skip)]
        reserved3: u8,
        /// Altitude code on 13 bits
        #[serde(rename = "altitude")]
        ac: AC13Field,
        /// Message, ACAS (56 bits, a BDS of a type requested in UF=0)
        #[deku(count = "7")]
        #[serde(skip)]
        mv: Vec<u8>,
        /// Address/Parity
        #[serde(rename = "icao24")]
        #[deku(ctx = "crc")]
        ap: IcaoParity,
    },

    #[deku(id = "17")]
    #[serde(rename = "ADSB")]
    /// Extended Squitter ADS-B, Downlink Format 17 (3.1.2.8.6)
    ExtendedSquitterADSB(ADSB),

    /// Extended Squitter Supplementary, Downlink Format 18 (3.1.2.8.7)
    ///
    /// Non-Transponder-based ADS-B Transmitting Subsystems and TIS-B Transmitting equipment.
    /// Equipment that cannot be interrogated.
    #[deku(id = "18")]
    #[serde(skip)]
    ExtendedSquitterTisB {
        /// Enum containing message
        cf: ControlField,
        /// Parity/interrogator identifier
        pi: ICAO,
    },

    /// DF=19: Extended Squitter Military Application, Downlink Format 19 (3.1.2.8.8)
    #[deku(id = "19")]
    #[serde(skip)]
    ExtendedSquitterMilitary {
        /// Reserved
        #[deku(bits = "3")]
        af: u8,
    },

    /// Comm-B Altitude Reply, Downlink Format 20 (3.1.2.6.6)
    #[deku(id = "20")]
    #[serde(rename = "DF20")]
    CommBAltitudeReply {
        /// Flight Status
        #[serde(skip)]
        fs: FlightStatus,
        /// Downlink Request
        #[serde(skip)]
        dr: DownlinkRequest,
        /// Utility Message
        #[serde(skip)]
        um: UtilityMessage,
        /// Altitude code on 13 bits
        #[serde(rename = "altitude")]
        ac: AC13Field,
        /// BDS Message, Comm-B
        #[serde(flatten)]
        bds: DataSelector,
        /// address/parity
        #[serde(rename = "icao24")]
        #[deku(ctx = "crc")]
        ap: IcaoParity,
    },

    /// Comm-B Identity Reply, Downlink Format 21 (3.1.2.6.8)
    #[deku(id = "21")]
    #[serde(rename = "DF21")]
    CommBIdentityReply {
        /// Flight Status
        #[serde(skip)]
        fs: FlightStatus,
        /// Downlink Request
        #[serde(skip)]
        dr: DownlinkRequest,
        /// Utility Message
        #[serde(skip)]
        um: UtilityMessage,
        /// Identity code (squawk)
        #[serde(rename = "squawk")]
        id: IdentityCode,
        /// BDS Message, Comm-B
        #[serde(flatten)]
        bds: DataSelector,
        /// Address/Parity
        #[serde(rename = "icao24")]
        #[deku(ctx = "crc")]
        ap: IcaoParity,
    },

    /// 24: Comm-D Extended, Downlink Format 24 (3.1.2.7.3)
    #[deku(id_pat = "24..=31")]
    CommDExtended {
        /// Reserved
        #[deku(bits = "1")]
        spare: u8,
        /// Control, ELM
        #[serde(skip)]
        ke: KE,
        /// Number of D-segment
        #[deku(bits = "4")]
        nd: u8,
        /// Message, Comm-D, 80 bits
        #[deku(count = "10")]
        md: Vec<u8>,
        /// Address/Parity
        parity: ICAO,
    },
}

/// The entry point to Mode S and ADS-B decoding
///
/// Use as `Message::from_bytes()` in mostly all applications
#[derive(Debug, PartialEq, serde::Serialize, DekuRead, Clone)]
pub struct Message {
    /// Calculated from all bits, should be 0 for ADS-B (raises a DekuError),
    /// icao24 otherwise
    #[deku(reader = "Self::read_crc(deku::input_bits)")]
    #[serde(skip)]
    pub crc: u32,

    /// The Downlink Format encoded in 5 bits
    #[serde(flatten)]
    #[deku(ctx = "*crc")]
    pub df: DF,
}

impl Message {
    /// Read rest as CRC bits
    fn read_crc(
        rest: &BitSlice<u8, Msb0>,
    ) -> Result<(&BitSlice<u8, Msb0>, u32), DekuError> {
        const MODES_LONG_MSG_BYTES: usize = 14;
        const MODES_SHORT_MSG_BYTES: usize = 7;

        let (_, remaining_bytes, _) = rest.domain().region().unwrap();

        // Decode the DF quickly to determine the length of the message
        let df = remaining_bytes[0] >> 3;

        let bit_len = if df & 0x10 != 0 {
            MODES_LONG_MSG_BYTES * 8
        } else {
            MODES_SHORT_MSG_BYTES * 8
        };

        let crc = modes_checksum(remaining_bytes, bit_len)?;
        // Also the CRC must be 0 for ADS-B (DF=17) messages
        match (df, crc) {
            (17, c) if c > 0 => Err(DekuError::Assertion(format!(
                "Invalid CRC in ADS-B message: {c}"
            ))),
            _ => Ok((rest, crc)),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let crc = self.crc;
        match &self.df {
            DF::ShortAirAirSurveillance { ac, .. } => {
                writeln!(f, " DF0. Short Air-Air Surveillance")?;
                writeln!(f, "  ICAO Address:  {crc:06x} (Mode S / ADS-B)")?;
                if ac.0 > 0 {
                    let altitude = ac.0;
                    writeln!(f, "  Air/Ground:    airborne")?;
                    writeln!(f, "  Altitude:      {altitude} ft barometric")?;
                } else {
                    writeln!(f, "  Air/Ground:    ground")?;
                }
            }
            DF::SurveillanceAltitudeReply { fs, ac, .. } => {
                writeln!(f, " DF4. Surveillance, Altitude Reply")?;
                writeln!(f, "  ICAO Address:  {crc:06x} (Mode S / ADS-B)")?;
                writeln!(f, "  Air/Ground:    {fs}")?;
                if ac.0 > 0 {
                    let altitude = ac.0;
                    writeln!(f, "  Altitude:      {altitude} ft barometric")?;
                }
            }
            DF::SurveillanceIdentityReply { fs, id, .. } => {
                writeln!(f, " DF5. Surveillance, Identity Reply")?;
                writeln!(f, "  ICAO Address:  {crc:06x} (Mode S / ADS-B)")?;
                writeln!(f, "  Air/Ground:    {fs}")?;
                writeln!(f, "  Squawk:        {id}")?;
            }
            DF::AllCallReply {
                capability, icao, ..
            } => {
                writeln!(f, " DF11. All Call Reply")?;
                writeln!(f, "  ICAO Address:  {icao} (Mode S / ADS-B)")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
            }
            DF::LongAirAirSurveillance { ac, .. } => {
                writeln!(f, " DF16. Long Air-Air ACAS")?;
                writeln!(f, "  ICAO Address:  {crc:06x} (Mode S / ADS-B)")?;
                if ac.0 > 0 {
                    let altitude = ac.0;
                    writeln!(f, "  Air/Ground:    airborne")?;
                    writeln!(f, "  Baro altitude: {altitude} ft")?;
                } else {
                    writeln!(f, "  Air/Ground:    ground")?;
                }
            }
            DF::ExtendedSquitterADSB(msg) => {
                write!(f, "{msg}")?;
            }
            DF::ExtendedSquitterTisB { cf, .. } => {
                // DF18
                write!(f, "{cf}")?;
            }
            DF::ExtendedSquitterMilitary { .. } => {} // DF19
            DF::CommBAltitudeReply { ac, bds, .. } => {
                writeln!(f, " DF20. Comm-B, Altitude Reply")?;
                writeln!(f, "  ICAO Address:  {crc:x?}")?;
                let altitude = ac.0;
                writeln!(f, "  Altitude:      {altitude} ft")?;
                write!(f, "  {bds}")?;
            }
            DF::CommBIdentityReply { id, bds, .. } => {
                writeln!(f, " DF21. Comm-B, Identity Reply")?;
                writeln!(f, "  ICAO Address:  {crc:x?}")?;
                writeln!(f, "  Squawk:        {id:x?}")?;
                write!(f, "    {bds}")?;
            }
            DF::CommDExtended { .. } => {
                writeln!(f, " DF24..=31 Comm-D Extended Length Message")?;
                writeln!(f, "  ICAO Address:     {crc:x?}")?;
            }
        }
        Ok(())
    }
}

/// ICAO 24-bit address, commonly use to reference airframes, i.e. tail numbers
/// of aircraft
#[derive(PartialEq, Eq, PartialOrd, DekuRead, Hash, Copy, Clone, Ord)]
#[deku(ctx = "crc: u32")]
pub struct IcaoParity(
    // Ok it looks convoluted, actually the final bits are already read when
    // we compute the crc so we don't need to read this again
    #[deku(bits = 24, map = "|_v: u32| -> Result<_, DekuError> { Ok(crc) }")]
    pub u32,
);

impl fmt::Debug for IcaoParity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:06x}", self.0)?;
        Ok(())
    }
}

impl fmt::Display for IcaoParity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:06x}", self.0)?;
        Ok(())
    }
}

impl Serialize for IcaoParity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let icao = format!("{:06x}", &self.0);
        serializer.serialize_str(&icao)
    }
}

impl core::str::FromStr for IcaoParity {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = u32::from_str_radix(s, 16)?;
        Ok(Self(num))
    }
}

/// ICAO 24-bit address, commonly use to reference airframes, i.e. tail numbers
/// of aircraft
#[derive(PartialEq, Eq, PartialOrd, DekuRead, Hash, Copy, Clone, Ord)]
pub struct ICAO(#[deku(bits = 24, endian = "big")] pub u32);

impl fmt::Debug for ICAO {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:06x}", self.0)?;
        Ok(())
    }
}

impl fmt::Display for ICAO {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:06x}", self.0)?;
        Ok(())
    }
}

impl Serialize for ICAO {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let icao = format!("{:06x}", &self.0);
        serializer.serialize_str(&icao)
    }
}

impl core::str::FromStr for ICAO {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = u32::from_str_radix(s, 16)?;
        Ok(Self(num))
    }
}
/// 13 bit identity code (squawk code), a 4-octal digit identifier
#[derive(PartialEq, DekuRead, Copy, Clone)]
pub struct IdentityCode(#[deku(reader = "Self::read(deku::rest)")] pub u16);

impl IdentityCode {
    fn read(
        rest: &BitSlice<u8, Msb0>,
    ) -> Result<(&BitSlice<u8, Msb0>, u16), DekuError> {
        let (rest, num) =
            u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(13)))?;
        Ok((rest, decode_id13(num)))
    }
}

impl fmt::Debug for IdentityCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}", self.0)?;
        Ok(())
    }
}

impl fmt::Display for IdentityCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}", self.0)?;
        Ok(())
    }
}

impl Serialize for IdentityCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let squawk = format!("{:04x}", &self.0);
        serializer.serialize_str(&squawk)
    }
}

/// 13 bit encoded altitude
#[derive(Debug, PartialEq, Eq, serde::Serialize, DekuRead, Copy, Clone)]
pub struct AC13Field(#[deku(reader = "Self::read(deku::rest)")] pub u16);

impl AC13Field {
    fn read(
        rest: &BitSlice<u8, Msb0>,
    ) -> Result<(&BitSlice<u8, Msb0>, u16), DekuError> {
        let (rest, ac13field) =
            u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(13)))?;

        let m_bit = ac13field & 0x0040;
        let q_bit = ac13field & 0x0010;

        if m_bit != 0 {
            let meters = ((ac13field & 0x1f80) >> 2) | (ac13field & 0x3f);
            // convert to ft
            Ok((rest, (meters as f32 * 3.28084) as u16))
        } else if q_bit != 0 {
            // 11 bit integer resulting from the removal of bit Q and M
            let n = ((ac13field & 0x1f80) >> 2)
                | ((ac13field & 0x0020) >> 1)
                | (ac13field & 0x000f);
            if n > 40 {
                Ok((rest, n * 25 - 1000)) // 25 ft interval
            } else {
                // TODO error?
                Ok((rest, 0))
            }
        } else {
            // 11 bit Gillham coded altitude
            if let Ok(n) = gray2alt(decode_id13(ac13field)) {
                Ok((rest, (100 * n) as u16))
            } else {
                Ok((rest, 0))
            }
        }
    }
}

/// Transponder level and additional information (3.1.2.5.2.2.1)
#[derive(Debug, PartialEq, serde::Serialize, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
#[allow(non_camel_case_types)]
pub enum Capability {
    /// Level 1 transponder (surveillance only), and either airborne or on the ground
    #[serde(rename = "level1")]
    AG_LEVEL1 = 0x00,
    #[deku(id_pat = "0x01..=0x03")]
    AG_RESERVED,
    /// Level 2 or above transponder, on ground
    #[serde(rename = "ground")]
    AG_GROUND = 0x04,
    /// Level 2 or above transponder, airborne
    #[serde(rename = "airborne")]
    AG_AIRBORNE = 0x05,
    /// Level 2 or above transponder, either airborne or on ground
    #[serde(rename = "ground/airborne")]
    AG_GROUND_AIRBORNE = 0x06,
    /// DR field is not equal to 0,
    /// or fs field equal 2, 3, 4, or 5,
    /// and either airborne or on ground
    AG_DR0 = 0x07,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::AG_LEVEL1 => "Level 1",
                Self::AG_RESERVED => "reserved",
                Self::AG_GROUND => "ground",
                Self::AG_AIRBORNE => "airborne",
                Self::AG_GROUND_AIRBORNE => "ground/airborne",
                Self::AG_DR0 => "DR0",
            }
        )
    }
}

/// Airborne or Ground and SPI (used in DF=4, 5, 20 or 21)
#[derive(Debug, PartialEq, serde::Serialize, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
#[serde(rename_all = "snake_case")]
pub enum FlightStatus {
    NoAlertNoSpiAirborne = 0b000,
    NoAlertNoSpiOnGround = 0b001,
    AlertNoSpiAirborne = 0b010,
    AlertNoSpiOnGround = 0b011,
    AlertSpiAirborneGround = 0b100,
    NoAlertSpiAirborneGround = 0b101,
    Reserved = 0b110,
    NotAssigned = 0b111,
}

impl fmt::Display for FlightStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::NoAlertNoSpiAirborne => "airborne",
                Self::AlertSpiAirborneGround
                | Self::NoAlertSpiAirborneGround => "airborne/ground",
                Self::NoAlertNoSpiOnGround => "ground",
                Self::AlertNoSpiAirborne => "airborne",
                Self::AlertNoSpiOnGround => "ground",
                _ => "reserved",
            }
        )
    }
}

/// The downlink request (used in DF=4, 5, 20 or 21)
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "5")]
pub enum DownlinkRequest {
    None = 0b00000,
    RequestSendCommB = 0b00001,
    CommBBroadcastMsg1 = 0b00100,
    CommBBroadcastMsg2 = 0b00101,
    #[deku(id_pat = "_")]
    Unknown,
}

/// The utility message (used in DF=4, 5, 20 or 21)
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
pub struct UtilityMessage {
    #[deku(bits = "4")]
    pub iis: u8,
    pub ids: UtilityMessageType,
}

/// The utility message type (used in DF=4, 5, 20 or 21)
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "2")]
pub enum UtilityMessageType {
    NoInformation = 0b00,
    CommB = 0b01,
    CommC = 0b10,
    CommD = 0b11,
}

/// The control field in TIS-B messages (DF=18)
#[derive(Debug, PartialEq, serde::Serialize, DekuRead, Clone)]
pub struct ControlField {
    t: ControlFieldType,
    /// AA: Address, Announced
    pub aa: ICAO,
    /// ME: message, extended squitter
    pub me: ME,
}

impl fmt::Display for ControlField {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

/// The control field type in TIS-B messages (DF=18)
#[derive(Debug, PartialEq, serde::Serialize, DekuRead, Clone)]
#[deku(type = "u8", bits = "3")]
#[allow(non_camel_case_types)]
pub enum ControlFieldType {
    /// ADS-B Message from a non-transponder device
    #[deku(id = "0")]
    ADSB_ES_NT,

    /// Reserved for ADS-B for ES/NT devices for alternate address space
    #[deku(id = "1")]
    ADSB_ES_NT_ALT,

    /// Code 2, Fine Format TIS-B Message
    #[deku(id = "2")]
    TISB_FINE,

    /// Code 3, Coarse Format TIS-B Message
    #[deku(id = "3")]
    TISB_COARSE,

    /// Code 4, Coarse Format TIS-B Message
    #[deku(id = "4")]
    TISB_MANAGE,

    /// Code 5, TIS-B Message for replay ADS-B Message
    ///
    /// Anonymous 24-bit addresses
    #[deku(id = "5")]
    TISB_ADSB_RELAY,

    /// Code 6, TIS-B Message, Same as DF=17
    #[deku(id = "6")]
    TISB_ADSB,

    /// Code 7, Reserved
    #[deku(id = "7")]
    Reserved,
}

impl fmt::Display for ControlFieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s_type = match self {
            Self::ADSB_ES_NT | Self::ADSB_ES_NT_ALT => "(ADS-B)",
            Self::TISB_COARSE | Self::TISB_ADSB_RELAY | Self::TISB_FINE => {
                "(TIS-B)"
            }
            Self::TISB_MANAGE | Self::TISB_ADSB => "(ADS-R)",
            Self::Reserved => "(unknown addressing scheme)",
        };
        write!(f, "{s_type}")
    }
}

/// Uplink / Downlink (DF=24)
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum KE {
    DownlinkELMTx = 0,
    UplinkELMAck = 1,
}

/// Decode a [Gillham code](https://en.wikipedia.org/wiki/Gillham_code)
/// 
/// In the squawk (identity) field bits are interleaved as follows in
/// (message bit 20 to bit 32):
///
/// C1-A1-C2-A2-C4-A4-ZERO-B1-D1-B2-D2-B4-D4
///
/// So every group of three bits A, B, C, D represent an integer from 0 to 7.
///
/// The actual meaning is just 4 octal numbers, but we convert it into a hex
/// number that happens to represent the four octal numbers.
///

#[rustfmt::skip]
pub fn decode_id13(id13_field: u16) -> u16 {
    let mut hex_gillham: u16 = 0;

    if id13_field & 0x1000 != 0 { hex_gillham |= 0x0010; } // Bit 12 = C1
    if id13_field & 0x0800 != 0 { hex_gillham |= 0x1000; } // Bit 11 = A1
    if id13_field & 0x0400 != 0 { hex_gillham |= 0x0020; } // Bit 10 = C2
    if id13_field & 0x0200 != 0 { hex_gillham |= 0x2000; } // Bit  9 = A2
    if id13_field & 0x0100 != 0 { hex_gillham |= 0x0040; } // Bit  8 = C4
    if id13_field & 0x0080 != 0 { hex_gillham |= 0x4000; } // Bit  7 = A4
    // if id13_field & 0x0040 != 0 {hex_gillham |= 0x0800;} // Bit  6 = X  or M
    if id13_field & 0x0020 != 0 { hex_gillham |= 0x0100; } // Bit  5 = B1
    if id13_field & 0x0010 != 0 { hex_gillham |= 0x0001; } // Bit  4 = D1 or Q
    if id13_field & 0x0008 != 0 { hex_gillham |= 0x0200; } // Bit  3 = B2
    if id13_field & 0x0004 != 0 { hex_gillham |= 0x0002; } // Bit  2 = D2
    if id13_field & 0x0002 != 0 { hex_gillham |= 0x0400; } // Bit  1 = B4
    if id13_field & 0x0001 != 0 { hex_gillham |= 0x0004; } // Bit  0 = D4

    hex_gillham
}

/// Convert a [Gillham code](https://en.wikipedia.org/wiki/Gillham_code) to
/// an altitude in feet.
#[rustfmt::skip]
pub fn gray2alt(gray: u16) -> Result<i32, &'static str> {
    let mut five_hundreds: u32 = 0;
    let mut one_hundreds: u32 = 0;

    // check zero bits are zero, D1 set is illegal; C1,,C4 cannot be Zero
    if (gray & 0x8889) != 0 || (gray & 0x00f0) == 0 {
        return Err("Invalid altitude");
    }

    if gray & 0x0010 != 0 { one_hundreds ^= 0x007; } // C1
    if gray & 0x0020 != 0 { one_hundreds ^= 0x003; } // C2
    if gray & 0x0040 != 0 { one_hundreds ^= 0x001; } // C4

    // Remove 7s from OneHundreds (Make 7->5, snd 5->7).
    if (one_hundreds & 5) == 5 { one_hundreds ^= 2; }

    // Check for invalid codes, only 1 to 5 are valid
    if one_hundreds > 5 { return Err("Invalid altitude"); }

    // if gray & 0x0001 {five_hundreds ^= 0x1FF;} // D1 never used for altitude
    if gray & 0x0002 != 0 { five_hundreds ^= 0x0ff; } // D2
    if gray & 0x0004 != 0 { five_hundreds ^= 0x07f; } // D4
    if gray & 0x1000 != 0 { five_hundreds ^= 0x03f; } // A1
    if gray & 0x2000 != 0 { five_hundreds ^= 0x01f; } // A2
    if gray & 0x4000 != 0 { five_hundreds ^= 0x00f; } // A4
    if gray & 0x0100 != 0 { five_hundreds ^= 0x007; } // B1
    if gray & 0x0200 != 0 { five_hundreds ^= 0x003; } // B2
    if gray & 0x0400 != 0 { five_hundreds ^= 0x001; } // B4

    // Correct order of one_hundreds.
    if five_hundreds & 1 != 0 && one_hundreds <= 6 {
        one_hundreds = 6 - one_hundreds;
    }

    let n = (five_hundreds * 5) + one_hundreds;
    if n >= 13 {
        Ok(n as i32 - 13)
    } else {
        Err("Invalid altitude")
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use hexlit::hex;

    #[test]
    fn test_ac13field() {
        let bytes = hex!("a0001910cc300030aa0000eae004");
        let msg = Message::from_bytes((&bytes, 0)).unwrap().1;
        match msg.df {
            DF::CommBAltitudeReply { ac, .. } => {
                assert_eq!(ac.0, 39000);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_invalid_crc() {
        let bytes = hex!("8d4ca251204994b1c36e60a5343d");
        let msg = Message::from_bytes((&bytes, 0));
        if let Err(e) = msg {
            match e {
                DekuError::Assertion(_msg) => (),
                _ => unreachable!(),
            }
        } else {
            unreachable!()
        }
    }
}
