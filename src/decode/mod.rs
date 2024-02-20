pub mod adsb;
pub mod bds;
pub mod crc;

use adsb::{Typecode, ADSB};
use bds::{decode_id13_field, mode_a_to_mode_c, BDS};
use crc::modes_checksum;
use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::fmt;

/**
# Downlink Format Support
|  [`DF`]  |  Name                               |  Section    |
| -------- | ----------------------------------- | ----------- |
| 0        | [`Short Air-Air Surveillance`]      | 3.1.2.8.2   |
| 4        | [`Surveillance Altitude Reply`]     | 3.1.2.6.5   |
| 5        | [`Surveillance Identity Reply`]     | 3.1.2.6.7   |
| 11       | [`All Call Reply`]                  | 2.1.2.5.2.2 |
| 16       | [`Long Air-Air Surveillance`]       | 3.1.2.8.3   |
| 17       | [`Extended Squitter(ADS-B)`]        | 3.1.2.8.6   |
| 18       | [`Extended Squitter(TIS-B)`]        | 3.1.2.8.7   |
| 19       | [`Extended Squitter(Military)`]     | 3.1.2.8.8   |
| 20       | [`Comm-B Altitude Reply`]           | 3.1.2.6.6   |
| 21       | [`Comm-B Identity Reply`]           | 3.1.2.6.8   |
| 24       | [`Comm-D`]                          | 3.1.2.7.3   |
 */

#[derive(Debug, PartialEq, DekuRead, Clone)]
#[deku(type = "u8", bits = "5")]
pub enum DF {
    #[deku(id = "0")]
    ShortAirAirSurveillance {
        /// VS: Vertical Status
        #[deku(bits = "1")]
        vs: u8,
        /// CC:
        #[deku(bits = "1")]
        cc: u8,
        /// Spare
        #[deku(bits = "1")]
        unused: u8,
        /// SL: Sensitivity level, ACAS
        #[deku(bits = "3")]
        sl: u8,
        /// Spare
        #[deku(bits = "2")]
        unused1: u8,
        /// RI: Reply Information
        #[deku(bits = "4")]
        ri: u8,
        /// Spare
        #[deku(bits = "2")]
        unused2: u8,
        /// AC: altitude code
        altitude: AC13Field,
        /// AP: address, parity
        parity: ICAO,
    },
    #[deku(id = "4")]
    SurveillanceAltitudeReply {
        /// FS: Flight Status
        fs: FlightStatus,
        /// DR: DownlinkRequest
        dr: DownlinkRequest,
        /// UM: Utility Message
        um: UtilityMessage,
        /// AC: AltitudeCode
        ac: AC13Field,
        /// AP: Address/Parity
        ap: ICAO,
    },
    #[deku(id = "5")]
    SurveillanceIdentityReply {
        /// FS: Flight Status
        fs: FlightStatus,
        /// DR: Downlink Request
        dr: DownlinkRequest,
        /// UM: UtilityMessage
        um: UtilityMessage,
        /// ID: Identity
        id: IdentityCode,
        /// AP: Address/Parity
        ap: ICAO,
    },
    /// 11: (Mode S) All-call reply, Downlink format 11 (2.1.2.5.2.2)
    #[deku(id = "11")]
    AllCallReply {
        /// CA: Capability
        capability: Capability,
        /// AA: Address Announced
        icao: ICAO,
        /// PI: Parity/Interrogator identifier
        p_icao: ICAO,
    },
    #[deku(id = "16")]
    LongAirAir {
        #[deku(bits = "1")]
        vs: u8,
        #[deku(bits = "2")]
        spare1: u8,
        #[deku(bits = "3")]
        sl: u8,
        #[deku(bits = "2")]
        spare2: u8,
        #[deku(bits = "4")]
        ri: u8,
        #[deku(bits = "2")]
        spare3: u8,
        /// AC: altitude code
        altitude: AC13Field,
        /// MV: message, acas
        #[deku(count = "7")]
        mv: Vec<u8>,
        /// AP: address, parity
        parity: ICAO,
    },
    #[deku(id = "17")]
    ADSB(ADSB),

    /// 18: Extended Squitter/Supplementary, Downlink Format 18 (3.1.2.8.7)
    ///
    /// Non-Transponder-based ADS-B Transmitting Subsystems and TIS-B Transmitting equipment.
    /// Equipment that cannot be interrogated.
    #[deku(id = "18")]
    TisB {
        /// Enum containing message
        cf: ControlField,
        /// PI: parity/interrogator identifier
        pi: ICAO,
    },

    /// 19: Extended Squitter Military Application, Downlink Format 19 (3.1.2.8.8)
    #[deku(id = "19")]
    ExtendedQuitterMilitaryApplication {
        /// Reserved
        #[deku(bits = "3")]
        af: u8,
    },

    #[deku(id = "20")]
    CommBAltitudeReply {
        /// FS: Flight Status
        flight_status: FlightStatus,
        /// DR: Downlink Request
        dr: DownlinkRequest,
        /// UM: Utility Message
        um: UtilityMessage,
        /// AC: Altitude Code
        alt: AC13Field,
        /// MB Message, Comm-B
        bds: BDS,
        /// AP: address/parity
        parity: ICAO,
    },
    #[deku(id = "21")]
    CommBIdentityReply {
        /// FS: Flight Status
        fs: FlightStatus,
        /// DR: Downlink Request
        dr: DownlinkRequest,
        /// UM: Utility Message
        um: UtilityMessage,
        /// ID: Identity
        #[deku(
            bits = "13",
            endian = "big",
            map = "|squawk: u32| -> Result<_, DekuError> {Ok(bds::decode_id13_field(squawk))}"
        )]
        id: u32,
        /// MB Message, Comm-B
        bds: BDS,
        /// AP address/parity
        parity: ICAO,
    },
    #[deku(id_pat = "24..=31")]
    CommDExtendedLengthMessage {
        /// Spare - 1 bit
        #[deku(bits = "1")]
        spare: u8,
        /// KE: control, ELM
        ke: KE,
        /// ND: number of D-segment
        #[deku(bits = "4")]
        nd: u8,
        /// MD: message, Comm-D, 80 bits
        #[deku(count = "10")]
        md: Vec<u8>,
        /// AP: address/parity
        parity: ICAO,
    },
}

/// Downlink ADS-B Packet
#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct Message {
    /// Starting with 5 bit identifier, decode packet
    pub df: DF,

    // Calculated from all bits, used as ICAO for Response packets
    #[deku(reader = "Self::read_crc(df, deku::input_bits)")]
    pub crc: u32,
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Message", 10)?;
        state.serialize_field("crc", &self.crc)?;
        match &self.df {
            DF::ADSB(adsb) => {
                state.serialize_field("adsb", &adsb)?;
            }
            _ => {}
        }

        state.end()
    }
}

impl Message {
    /// Read rest as CRC bits
    fn read_crc<'b>(
        df: &DF,
        rest: &'b BitSlice<u8, Msb0>,
    ) -> Result<(&'b BitSlice<u8, Msb0>, u32), DekuError> {
        const MODES_LONG_MSG_BYTES: usize = 14;
        const MODES_SHORT_MSG_BYTES: usize = 7;

        let bit_len = if let Ok(id) = df.deku_id() {
            if id & 0x10 != 0 {
                MODES_LONG_MSG_BYTES * 8
            } else {
                MODES_SHORT_MSG_BYTES * 8
            }
        } else {
            // In this case, it's the DF::CommD, which has multiple ids
            MODES_LONG_MSG_BYTES * 8
        };

        let (_, remaining_bytes, _) = rest.domain().region().unwrap();
        let crc = modes_checksum(remaining_bytes, bit_len)?;
        Ok((rest, crc))
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let crc = self.crc;
        match &self.df {
            DF::ShortAirAirSurveillance { altitude, .. } => {
                writeln!(f, " DF0. Short Air-Air Surveillance")?;
                writeln!(f, "  ICAO Address:  {crc:06x} (Mode S / ADS-B)")?;
                if altitude.0 > 0 {
                    let altitude = altitude.0;
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
                let identity = id.0;
                writeln!(f, " DF5. Surveillance, Identity Reply")?;
                writeln!(f, "  ICAO Address:  {crc:06x} (Mode S / ADS-B)")?;
                writeln!(f, "  Air/Ground:    {fs}")?;
                writeln!(f, "  Squawk:        {identity:04x}")?;
            }
            DF::AllCallReply {
                capability, icao, ..
            } => {
                writeln!(f, " DF11. All Call Reply")?;
                writeln!(f, "  ICAO Address:  {icao} (Mode S / ADS-B)")?;
                writeln!(f, "  Air/Ground:    {capability}")?;
            }
            DF::LongAirAir { altitude, .. } => {
                writeln!(f, " DF16. Long Air-Air ACAS")?;
                writeln!(f, "  ICAO Address:  {crc:06x} (Mode S / ADS-B)")?;
                // TODO the airborne? should't be static
                if altitude.0 > 0 {
                    let altitude = altitude.0;
                    writeln!(f, "  Air/Ground:    airborne?")?;
                    writeln!(f, "  Baro altitude: {altitude} ft")?;
                } else {
                    writeln!(f, "  Air/Ground:    ground")?;
                }
            }
            DF::ADSB(adsb_msg) => {
                write!(f, "{}", adsb_msg.to_string()?)?;
            }
            DF::TisB { cf, .. } => {
                // DF18
                write!(f, "{cf}")?;
            }
            // TODO
            DF::ExtendedQuitterMilitaryApplication { .. } => {} // DF19
            DF::CommBAltitudeReply { bds, alt, .. } => {
                writeln!(f, " DF20. Comm-B, Altitude Reply")?;
                writeln!(f, "  ICAO Address:  {crc:x?} (Mode S / ADS-B)")?;
                let altitude = alt.0;
                writeln!(f, "  Altitude:      {altitude} ft")?;
                write!(f, "  {bds}")?;
            }
            DF::CommBIdentityReply { id, bds, .. } => {
                writeln!(f, " DF21. Comm-B, Identity Reply")?;
                writeln!(f, "    ICAO Address:  {crc:x?} (Mode S / ADS-B)")?;
                writeln!(f, "    Squawk:        {id:x?}")?;
                write!(f, "    {bds}")?;
            }
            DF::CommDExtendedLengthMessage { .. } => {
                writeln!(f, " DF24..=31 Comm-D Extended Length Message")?;
                writeln!(f, "    ICAO Address:     {crc:x?} (Mode S / ADS-B)")?;
            }
        }
        Ok(())
    }
}

/// ICAO Address; Mode S transponder code
#[derive(Debug, PartialEq, Eq, PartialOrd, DekuRead, Hash, Copy, Clone, Ord)]
pub struct ICAO(pub [u8; 3]);

impl fmt::Display for ICAO {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}", self.0[0])?;
        write!(f, "{:02x}", self.0[1])?;
        write!(f, "{:02x}", self.0[2])?;
        Ok(())
    }
}

impl Serialize for ICAO {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let icao = format!("{}", &self);
        serializer.serialize_str(&icao)
    }
}

impl core::str::FromStr for ICAO {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = u32::from_str_radix(s, 16)?;
        let bytes = num.to_be_bytes();
        let num = [bytes[1], bytes[2], bytes[3]];
        Ok(Self(num))
    }
}

/// 13 bit identity code
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
pub struct IdentityCode(#[deku(reader = "Self::read(deku::rest)")] pub u16);

impl IdentityCode {
    fn read(rest: &BitSlice<u8, Msb0>) -> Result<(&BitSlice<u8, Msb0>, u16), DekuError> {
        let (rest, num) = u32::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(13)))?;

        let c1 = (num & 0b1_0000_0000_0000) >> 12;
        let a1 = (num & 0b0_1000_0000_0000) >> 11;
        let c2 = (num & 0b0_0100_0000_0000) >> 10;
        let a2 = (num & 0b0_0010_0000_0000) >> 9;
        let c4 = (num & 0b0_0001_0000_0000) >> 8;
        let a4 = (num & 0b0_0000_1000_0000) >> 7;
        let b1 = (num & 0b0_0000_0010_0000) >> 5;
        let d1 = (num & 0b0_0000_0001_0000) >> 4;
        let b2 = (num & 0b0_0000_0000_1000) >> 3;
        let d2 = (num & 0b0_0000_0000_0100) >> 2;
        let b4 = (num & 0b0_0000_0000_0010) >> 1;
        let d4 = num & 0b0_0000_0000_0001;

        let a = a4 << 2 | a2 << 1 | a1;
        let b = b4 << 2 | b2 << 1 | b1;
        let c = c4 << 2 | c2 << 1 | c1;
        let d = d4 << 2 | d2 << 1 | d1;

        let num: u16 = (a << 12 | b << 8 | c << 4 | d) as u16;
        Ok((rest, num))
    }
}

/// 13 bit encoded altitude
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
pub struct AC13Field(#[deku(reader = "Self::read(deku::rest)")] pub u16);

impl AC13Field {
    // TODO Add unit
    fn read(rest: &BitSlice<u8, Msb0>) -> Result<(&BitSlice<u8, Msb0>, u16), DekuError> {
        let (rest, num) = u32::read(rest, (deku::ctx::Endian::Big, deku::ctx::BitSize(13)))?;

        let m_bit = num & 0x0040;
        let q_bit = num & 0x0010;

        if m_bit != 0 {
            // TODO: this might be wrong?
            Ok((rest, 0))
        } else if q_bit != 0 {
            let n = ((num & 0x1f80) >> 2) | ((num & 0x0020) >> 1) | (num & 0x000f);
            let n = n * 25;
            if n > 1000 {
                Ok((rest, (n - 1000) as u16))
            } else {
                // TODO: add error
                Ok((rest, 0))
            }
        } else {
            // TODO 11 bit gillham coded altitude
            if let Ok(n) = mode_a_to_mode_c(decode_id13_field(num)) {
                Ok((rest, (100 * n) as u16))
            } else {
                Ok((rest, 0))
            }
        }
    }
}

/// Transponder level and additional information (3.1.2.5.2.2.1)
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
#[allow(non_camel_case_types)]
pub enum Capability {
    /// Level 1 transponder (surveillance only), and either airborne or on the ground
    AG_LEVEL1 = 0x00,
    #[deku(id_pat = "0x01..=0x03")]
    AG_RESERVED,
    /// Level 2 or above transponder, on ground
    AG_GROUND = 0x04,
    /// Level 2 or above transponder, airborne
    AG_AIRBORNE = 0x05,
    /// Level 2 or above transponder, either airborne or on ground
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

/// Airborne / Ground and SPI
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
pub enum FlightStatus {
    NoAlertNoSPIAirborne = 0b000,
    NoAlertNoSPIOnGround = 0b001,
    AlertNoSPIAirborne = 0b010,
    AlertNoSPIOnGround = 0b011,
    AlertSPIAirborneGround = 0b100,
    NoAlertSPIAirborneGround = 0b101,
    Reserved = 0b110,
    NotAssigned = 0b111,
}

impl fmt::Display for FlightStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::NoAlertNoSPIAirborne
                | Self::AlertSPIAirborneGround
                | Self::NoAlertSPIAirborneGround => "airborne?",
                Self::NoAlertNoSPIOnGround => "ground?",
                Self::AlertNoSPIAirborne => "airborne",
                Self::AlertNoSPIOnGround => "ground",
                _ => "reserved",
            }
        )
    }
}

/// Type of `DownlinkRequest`
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

#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
pub struct UtilityMessage {
    #[deku(bits = "4")]
    pub iis: u8,
    pub ids: UtilityMessageType,
}

/// Message Type
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "2")]
pub enum UtilityMessageType {
    NoInformation = 0b00,
    CommB = 0b01,
    CommC = 0b10,
    CommD = 0b11,
}

/// Control Field (B.3) for [`crate::DF::TisB`]
///
/// reference: ICAO 9871
#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct ControlField {
    t: ControlFieldType,
    /// AA: Address, Announced
    pub aa: ICAO,
    /// ME: message, extended squitter
    pub me: Typecode,
}

impl fmt::Display for ControlField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.me
                .to_string(self.aa, &format!("{}", self.t), Capability::AG_DR0, false,)?
        )
    }
}

#[derive(Debug, PartialEq, Eq, DekuRead, Clone)]
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
            Self::TISB_COARSE | Self::TISB_ADSB_RELAY | Self::TISB_FINE => "(TIS-B)",
            Self::TISB_MANAGE | Self::TISB_ADSB => "(ADS-R)",
            Self::Reserved => "(unknown addressing scheme)",
        };
        write!(f, "{s_type}")
    }
}

/// Uplink / Downlink
#[derive(Debug, PartialEq, Eq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum KE {
    DownlinkELMTx = 0,
    UplinkELMAck = 1,
}
