extern crate alloc;

use alloc::fmt;
use deku::prelude::*;
use serde::Serialize;

/// Structure is different according to ADS-B Version
/// Version 0 never send these messages
/// Version 1 and 2 have a different structure
#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
#[serde(untagged)]
pub enum OperationStatus {
    #[deku(id = "0")]
    Airborne(OperationStatusAirborne),

    #[deku(id = "1")]
    Surface(OperationStatusSurface),

    #[serde(skip)]
    #[deku(id_pat = "2..=7")]
    Reserved(#[deku(bits = "5")] u8, [u8; 5]),
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct OperationStatusAirborne {
    /// CC (16 bits)
    //#[serde(flatten)]
    #[serde(skip)] // TODO optional
    pub capability_class: CapabilityClassAirborne,

    /// OM
    #[serde(skip)]
    pub operational_mode: OperationalMode,

    #[deku(pad_bytes_before = "1")]
    #[serde(flatten)]
    // reserved: OM last 8 bits (diff for airborne/surface)
    pub version: ADSBVersionAirborne,
}

// TODO implement ADSBVersionAirborne
impl fmt::Display for OperationStatusAirborne {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "   Capability classes: {}", self.capability_class)?;
        writeln!(f, "   Operational modes:  {}", self.operational_mode)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct CapabilityClassAirborne {
    #[deku(bits = "2", assert_eq = "0")]
    #[serde(skip)]
    pub reserved0: u8,

    /// TCAS Resolution Advisory Active
    #[deku(bits = "1")]
    #[serde(rename = "ACAS")]
    pub acas: bool,

    /// 1090ES IN
    /// Cockpit Display of Traffic Information
    #[deku(bits = "1")]
    #[serde(rename = "CDTI")]
    pub cdti: bool,

    #[deku(bits = "2", assert_eq = "0")]
    #[serde(skip)]
    pub reserved1: u8,

    /// Air-Referenced Velocity Report Capability
    #[deku(bits = "1")]
    #[serde(rename = "ARV")]
    pub arv: bool,

    #[deku(bits = "1")]
    #[serde(rename = "TS")]
    /// Target State Report Capability
    pub ts: bool,

    #[deku(bits = "2")]
    #[deku(pad_bits_after = "6")] //reserved
    #[serde(rename = "TC")]
    /// Target Trajectory Change Report Capability
    /// 0: No capability for Trajectory Change Reports
    /// 1: Support for TC+0 reports only
    /// 2: Support for multiple TC reports
    /// 3: Reserved
    pub tc: u8,
}

impl fmt::Display for CapabilityClassAirborne {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.acas {
            write!(f, " ACAS")?;
        }
        if self.cdti {
            write!(f, " CDTI")?;
        }
        if self.arv {
            write!(f, " ARV")?;
        }
        if self.ts {
            write!(f, " TS")?;
        }
        if self.tc == 1 {
            write!(f, " TC")?;
        }
        Ok(())
    }
}

/// Version 2 support only
#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct OperationStatusSurface {
    /// CC (14 bits)
    //#[serde(flatten)]
    #[serde(skip)]
    pub capability_class: CapabilityClassSurface,

    /// CC L/W codes
    #[deku(bits = "4")]
    #[serde(skip)]
    pub lw_codes: u8,

    /// OM
    #[serde(skip)]
    pub operational_mode: OperationalMode,

    /// OM last 8 bits (diff for airborne/surface)
    // http://www.anteni.net/adsb/Doc/1090-WP30-18-DRAFT_DO-260B-V42.pdf
    // 2.2.3.2.7.2.4.7 “GPS Antenna Offset”
    // OM Code Subfield in Aircraft Operational Status Messages
    #[serde(skip)]
    pub gps_antenna_offset: u8,

    #[serde(flatten)]
    pub version: ADSBVersionSurface,
}

// TODO implement ADSBVersionAirborne
impl fmt::Display for OperationStatusSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.capability_class)?;
        write!(f, "   Operational modes: {}", self.operational_mode)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct CapabilityClassSurface {
    /// 0, 0 in current version, reserved as id for later versions
    #[deku(bits = "2", assert_eq = "0")]
    #[serde(skip)]
    pub reserved0: u8,

    /// Position Offset Applied
    #[deku(bits = "1")]
    pub poe: bool,

    /// Aircraft has ADS-B 1090ES Receive Capability
    #[deku(bits = "1")]
    #[deku(pad_bits_after = "2")] // reserved
    #[serde(rename = "1090ES")]
    pub es1090: bool,

    /// Class B2 Ground Vehicle transmitting with less than 70 watts
    #[deku(bits = "1")]
    #[serde(rename = "GRND")]
    pub b2_low: bool,

    /// Aircraft has ADS-B UAT Receive Capability
    #[deku(bits = "1")]
    #[serde(rename = "UATin")]
    pub uat_in: bool,

    /// Navigation Accuracy Category for Velocity, version 1,2
    #[deku(bits = "3")] // ME 11-13
    #[serde(rename = "NACv")]
    pub nac_v: u8,

    /// NIC Supplement used on the Surface
    #[deku(bits = "1")]
    #[serde(rename = "NICc")]
    pub nic_c: u8,
}

impl fmt::Display for CapabilityClassSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "   NICc:               {}", self.nic_c)?;
        writeln!(f, "   NACv:               {}", self.nac_v)?;
        Ok(())
    }
}

/// `OperationMode` field not including the last 8 bits that are different for Surface/Airborne
#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct OperationalMode {
    /// (0, 0) in Version 2, reserved for other values
    #[deku(bits = "2", assert_eq = "0")]
    #[serde(skip)]
    reserved: u8,

    #[deku(bits = "1")]
    tcas_ra_active: bool,

    #[deku(bits = "1")]
    ident_switch_active: bool,

    #[deku(bits = "1")]
    reserved_recv_atc_service: bool,

    #[deku(bits = "1")]
    single_antenna_flag: bool,

    #[deku(bits = "2")]
    system_design_assurance: u8,
}

impl fmt::Display for OperationalMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.tcas_ra_active {
            write!(f, " TCAS")?;
        }
        if self.ident_switch_active {
            write!(f, " IDENT_SWITCH_ACTIVE")?;
        }
        if self.reserved_recv_atc_service {
            write!(f, " ATC")?;
        }
        if self.single_antenna_flag {
            write!(f, " SAF")?;
        }
        if self.system_design_assurance != 0 {
            write!(f, " SDA={}", self.system_design_assurance)?;
        }
        Ok(())
    }
}

/// ADS-B Defined from different ICAO documents
///
/// reference: ICAO 9871 (5.3.2.3)
#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
#[serde(tag = "version")]
pub enum ADSBVersionAirborne {
    #[deku(id = "0")]
    #[serde(skip)] // useless, never happens
    DOC9871AppendixA(Empty),
    #[deku(id = "1")]
    #[serde(rename = "1")]
    DOC9871AppendixB(AirborneV1),
    #[deku(id = "2")]
    #[serde(rename = "2")]
    DOC9871AppendixC(AirborneV2),
    #[deku(id_pat = "3..=7")]
    #[serde(skip)]
    Reserved(Empty),
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct AirborneV1 {
    #[deku(bits = "1")]
    #[serde(rename = "NICs")]
    /// NIC Supplement bit (NICs)
    pub nic_s: u8,

    #[deku(bits = "4")] // ME 45-48
    #[serde(rename = "NACp")]
    /// Navigation Accuracy Category for position
    pub nac_p: u8,

    #[deku(bits = "2")]
    #[serde(rename = "BAQ")]
    pub barometric_altitude_quality: u8,

    #[deku(bits = "2")] // ME 51-52
    #[serde(rename = "SIL")]
    /// Surveillance Integrity Level (SIL)
    pub sil: u8,

    #[deku(bits = "1")]
    #[serde(rename = "BAI")]
    pub barometric_altitude_integrity: u8,

    #[deku(bits = "1")]
    #[deku(pad_bits_after = "2")] // reserved
    #[serde(rename = "HRD")]
    // 1 for magnetic, 0 for true north
    pub horizontal_reference_direction: u8,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct AirborneV2 {
    #[deku(bits = "1")]
    #[serde(rename = "NICa")]
    /// NIC supplement A (NICs)
    pub nic_a: u8,

    #[deku(bits = "4")] // ME 45-48
    #[serde(rename = "NACp")]
    /// Navigation Accuracy Category for position
    pub nac_p: u8,

    #[deku(bits = "2")]
    #[serde(rename = "GVA")]
    pub geometry_vertical_accuracy: u8,

    #[deku(bits = "2")] // ME 51-52
    #[serde(rename = "SIL")]
    /// Surveillance Integrity Level (SIL)
    pub sil: u8,

    #[deku(bits = "1")]
    #[serde(rename = "BAI")]
    pub barometric_altitude_integrity: u8,

    #[deku(bits = "1")]
    #[serde(rename = "HRD")]
    // 1 for magnetic, 0 for true north
    pub horizontal_reference_direction: u8,

    #[deku(bits = "1")] // ME 55, Version 2
    #[deku(pad_bits_after = "1")] // reserved
    #[serde(rename = "SILs")]
    /// SIL supplement bit
    /// 0 per hour
    /// 1 per sample
    pub sil_s: u8,
}

/// ADS-B Defined from different ICAO documents
///
/// reference: ICAO 9871 (5.3.2.3)
#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
#[serde(tag = "version")]
pub enum ADSBVersionSurface {
    #[deku(id = "0")]
    #[serde(skip)] // useless, never happens
    DOC9871AppendixA(Empty),
    #[deku(id = "1")]
    #[serde(rename = "1")]
    DOC9871AppendixB(SurfaceV1),
    #[deku(id = "2")]
    #[serde(rename = "2")]
    DOC9871AppendixC(SurfaceV2),
    #[deku(id_pat = "3..=7")]
    #[serde(skip)]
    Reserved(Empty),
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct SurfaceV1 {
    #[deku(bits = "1")] // ME 44
    #[serde(rename = "NICs")]
    /// NIC supplement bit
    pub nic_s: u8,

    #[deku(bits = "4")] // ME 45-48
    #[deku(pad_bits_after = "2")] // reserved
    #[serde(rename = "NACp")]
    /// Navigation Accuracy Category for position, version 1,2
    pub nac_p: u8,

    #[deku(bits = "2")] // ME 51-52
    #[serde(rename = "SIL")]
    /// Surveillance Integrity Level (SIL)
    pub sil: u8,

    #[deku(bits = "1")]
    #[serde(rename = "TAH")]
    pub track_angle_or_heading: u8,

    #[deku(bits = "1")]
    #[deku(pad_bits_after = "2")] // reserved
    #[serde(rename = "HRD")]
    // 1 for magnetic, 0 for true north
    pub horizontal_reference_direction: u8,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct SurfaceV2 {
    #[deku(bits = "1")] // ME 44
    #[serde(rename = "NICa")]
    /// NIC suppelement A
    pub nic_a: u8,

    #[deku(bits = "4")] // ME 45-48
    #[deku(pad_bits_after = "2")] // reserved
    #[serde(rename = "NACp")]
    /// Navigation Accuracy Category for position
    pub nac_p: u8,

    #[deku(bits = "2")] // ME 51-52
    #[serde(rename = "SIL")]
    /// Surveillance Integrity Level (SIL)
    pub sil: u8,

    #[deku(bits = "1")]
    #[serde(rename = "TAH")]
    pub track_angle_or_heading: u8,

    #[deku(bits = "1")]
    #[serde(rename = "HRD")]
    // 1 for magnetic, 0 for true north
    pub horizontal_reference_direction: u8,

    #[deku(bits = "1")] // ME 55
    #[deku(pad_bits_after = "1")] // reserved
    #[serde(rename = "SILs")]
    /// SIL suppelement bit, only version 2
    /// 0 per hour
    /// 1 per sample
    pub sil_supplement: u8,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct Empty {}
