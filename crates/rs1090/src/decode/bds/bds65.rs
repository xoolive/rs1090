use deku::prelude::*;
use serde::Serialize;
use std::fmt;

/**
 * ## Aircraft Operational Status (BDS 6,5 / TYPE=31)
 *
 * Extended Squitter ADS-B message providing operational capabilities and modes.  
 * Per ICAO Doc 9871 Table B-2-101: BDS code 6,5 — Extended squitter aircraft operational status
 *
 * Purpose: To provide the capability class and current operational mode of
 * ATC-related applications and other operational information.
 *
 * Message Structure (56 bits):
 * | TYPE | SUBTYPE | CC    | LW  | OM   | VERSION | NIC | NAC_P | BAQ | SIL | NIC_BARO | TRK_HDG | HRD | RES |
 * |------|---------|-------|-----|------|---------|-----|-------|-----|-----|----------|---------|-----|-----|
 * | 5    | 3       | 16    | 4   | 16   | 3       | 1   | 4     | 1   | 2   | 1        | 1       | 1   | 2   |
 *
 * Field Encoding per ICAO Doc 9871 §B.2.3.10:
 *
 * **TYPE Code** (bits 1-5): Fixed value 31 (11111 binary)
 *
 * **Subtype** (bits 6-8): 3-bit subtype field
 *   - 0 = Airborne Status Message
 *   - 1 = Surface Status Message
 *   - 2-7 = Reserved
 *
 * **Capability Class (CC) Codes** (bits 9-24): 16-bit capability field
 *   - Airborne (Subtype 0): ACAS, CDTI, ARV, TS, TC capabilities
 *     * Bits 9-10: Reserved (0)
 *     * Bit 11: ACAS (TCAS Resolution Advisory Active)
 *     * Bit 12: CDTI (Cockpit Display of Traffic Information)
 *     * Bits 13-14: Reserved (0)
 *     * Bit 15: ARV (Air-Referenced Velocity Report Capability)
 *     * Bit 16: TS (Target State Report Capability)
 *     * Bits 17-18: TC (Target Trajectory Change Report Capability)
 *       - 0 = No capability
 *       - 1 = TC+0 reports only
 *       - 2 = Multiple TC reports
 *       - 3 = Reserved
 *     * Bits 19-24: Reserved
 *   - Surface (Subtype 1): Similar capabilities for surface operations
 *
 * **Length/Width (L/W) Codes** (bits 25-28): 4-bit aircraft size (Surface only)
 *   - Encodes aircraft dimensions per §B.2.3.10.11
 *
 * **Operational Mode (OM) Codes** (bits 29-44): 16-bit operational mode field
 *   - Various operational flags and modes per §B.2.3.10.4
 *
 * **Version Number** (bits 45-47): 3-bit ADS-B version
 *   - 0 = Not implemented (DO-260)
 *   - 1 = DO-260A
 *   - 2 = DO-260B
 *   - 3-7 = Reserved for future versions
 *   - Note: Message structure differs significantly between versions
 *
 * **NIC Supplement** (bit 48): Navigation Integrity Category supplement
 *   - Used with NIC from position messages
 *
 * **NAC_P** (bits 49-52): Navigation Accuracy Category - Position
 *   - 4-bit horizontal position accuracy indicator
 *
 * **BAQ** (bit 53): Barometric Altitude Quality (when 0, bits 54-56 reserved)
 *
 * **SIL** (bits 54-55): Source Integrity Level (2 bits)
 *   - Probability of position integrity
 *
 * **NIC_BARO** (bit 56): Barometric altitude cross-check flag
 *
 * **TRK/HDG** (bit 57): Track angle vs. heading (Airborne)
 *   - 0 = True track angle
 *   - 1 = Magnetic heading
 *
 * **HRD** (bit 58): Heading Reference Direction (Airborne)
 *   - 0 = True north
 *   - 1 = Magnetic north
 *
 * **Reserved** (bits 59-56): Reserved for future use
 *
 * Version Differences:
 * - Version 0 (DO-260): Not implemented in practice, basic structure
 * - Version 1 (DO-260A): Added accuracy and integrity indicators
 * - Version 2 (DO-260B): Enhanced with additional operational modes
 *
 * Transmission:
 * - Message delivery accomplished using event-driven protocol
 * - Broadcast when capability or operational status changes
 *
 * Reference: ICAO Doc 9871 Table B-2-101, §B.2.3.10  
 * Additional details: DO-260B §2.2.3.2.7.2
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "3")]
#[serde(untagged)]
pub enum AircraftOperationStatus {
    #[deku(id = "0")]
    Airborne(OperationStatusAirborne),

    #[deku(id = "1")]
    Surface(OperationStatusSurface),

    #[deku(id_pat = "2..=7")]
    Reserved { id: u8, status: ReservedStatus },
}

impl fmt::Display for AircraftOperationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Aircraft Operation Status (BDS 6,5)")?;
        match &self {
            Self::Airborne(airborne) => write!(f, "{airborne}"),
            Self::Surface(surface) => write!(f, "{surface}"),
            Self::Reserved { id, status } => {
                write!(f, " Reserved: id={}, data={:?}", id, status.data)
            }
        }
    }
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct OperationStatusAirborne {
    /// The capacity class
    #[serde(skip)]
    pub capability_class: CapabilityClassAirborne,

    /// The operational mode
    #[serde(skip)]
    pub operational_mode: OperationalMode,

    #[deku(pad_bytes_before = "1")]
    #[serde(flatten)]
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
    /// - 0: No capability for Trajectory Change Reports
    /// - 1: Support for TC+0 reports only
    /// - 2: Support for multiple TC reports
    /// - 3: Reserved
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
    /// The capacity class
    #[serde(skip)]
    pub capability_class: CapabilityClassSurface,

    /// The capacity class L/W codes
    #[deku(bits = "4")]
    #[serde(skip)]
    pub lw_codes: u8,

    /// The operational mode
    #[serde(skip)]
    pub operational_mode: OperationalMode,

    /// The GPS antenna offset (2.2.3.2.7.2.4.7).
    /// Reference: <http://www.anteni.net/adsb/Doc/1090-WP30-18-DRAFT_DO-260B-V42.pdf>
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

    /// Navigation Accuracy Category for Velocity (NACv), only for version 1 and 2
    #[deku(bits = "3")] // ME 11-13
    #[serde(rename = "NACv")]
    pub nac_v: u8,

    /// NIC Supplement (NICc) used on the Surface
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

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct OperationalMode {
    #[deku(bits = "2", assert_eq = "0")]
    #[serde(skip)]
    reserved: u8,

    /// TCAS RA active
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

/// ADS-B version as defined from different ICAO documents.
/// Reference: ICAO 9871 (5.3.2.3)
///
/// There are three ADS-B versions implemented so far, starting from version 0
/// (specification defined in RTCA document DO-260). Version 1 was introduced
/// around 2008 (DO-260A), and version 2 around 2012 (DO-260B). Version 3 is
/// currently being developed.
#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "3")]
#[serde(tag = "version")]
pub enum ADSBVersionAirborne {
    #[deku(id = "0")]
    #[serde(rename = "0")] // useless, never happens
    /// ADS-B version 0 (BDS 6,5 undefined, so these messages should not happen)
    DOC9871AppendixA(Empty),
    #[deku(id = "1")]
    #[serde(rename = "1")]
    /// ADS-B version 1 (2008)
    DOC9871AppendixB(AirborneV1),
    #[deku(id = "2")]
    #[serde(rename = "2")]
    /// ADS-B version 2 (2012)
    DOC9871AppendixC(AirborneV2),
    #[deku(id_pat = "3..=7")]
    #[serde(rename = "3to7")]
    Reserved { id: u8 },
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct AirborneV1 {
    #[deku(bits = "1")]
    #[serde(rename = "NICs")]
    /// NIC Supplement bit (NICs)
    pub nic_s: u8,

    #[deku(bits = "4")]
    #[serde(rename = "NACp")]
    /// Navigation Accuracy Category for position (NACp)
    pub nac_p: u8,

    #[deku(bits = "2")]
    #[serde(rename = "BAQ")]
    /// Barometric Altitude Quality (BAQ)
    pub barometric_altitude_quality: u8,

    #[deku(bits = "2")]
    #[serde(rename = "SIL")]
    /// Surveillance Integrity Level (SIL)
    pub sil: u8,

    #[deku(bits = "1")]
    #[serde(rename = "BAI")]
    /// Barometric Altitude Integrity (BAI)
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
    /// NIC supplement A (NICa)
    pub nic_a: u8,

    #[deku(bits = "4")]
    #[serde(rename = "NACp")]
    /// Navigation Accuracy Category for position (NACp)
    pub nac_p: u8,

    #[deku(bits = "2")]
    #[serde(rename = "GVA")]
    /// Geometry Vertical Accuracy (GVA)
    pub geometry_vertical_accuracy: u8,

    #[deku(bits = "2")] // ME 51-52
    #[serde(rename = "SIL")]
    /// Surveillance Integrity Level (SIL)
    pub sil: u8,

    #[deku(bits = "1")]
    #[serde(rename = "BAI")]
    /// Barometric Altitude Integrity (BAI)
    pub barometric_altitude_integrity: u8,

    #[deku(bits = "1")]
    #[serde(rename = "HRD")]
    // 1 for magnetic, 0 for true north
    pub horizontal_reference_direction: u8,

    #[deku(bits = "1")]
    #[deku(pad_bits_after = "1")]
    #[serde(rename = "SILs")]
    /// SIL supplement bit, only in version 2:
    /// 0 means per hour,
    /// 1 means per sample.
    pub sil_s: u8,
}

/// ADS-B version as defined from different ICAO documents.  
/// Reference: ICAO 9871 (5.3.2.3)
///
/// There are three ADS-B versions implemented so far, starting from version 0
/// (specification defined in RTCA document DO-260).  
/// Version 1 was introduced around 2008 (DO-260A), and version 2 around 2012 (DO-260B).
/// Version 3 is currently being developed.
#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "3")]
#[serde(tag = "version")]
pub enum ADSBVersionSurface {
    #[deku(id = "0")]
    #[serde(rename = "0")]
    /// ADS-B version 0 (BDS 6,5 undefined, so these messages should not happen)
    DOC9871AppendixA(Empty),
    #[deku(id = "1")]
    #[serde(rename = "1")]
    /// ADS-B version 1 (2008)
    DOC9871AppendixB(SurfaceV1),
    #[deku(id = "2")]
    #[serde(rename = "2")]
    /// ADS-B version 2 (2012)
    DOC9871AppendixC(SurfaceV2),
    #[deku(id_pat = "3..=7")]
    #[serde(rename = "3to7")]
    Reserved { id: u8 },
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct SurfaceV1 {
    #[deku(bits = "1")]
    #[serde(rename = "NICs")]
    /// NIC supplement bit (NICs)
    pub nic_s: u8,

    #[deku(bits = "4")]
    #[deku(pad_bits_after = "2")] // reserved
    #[serde(rename = "NACp")]
    /// Navigation Accuracy Category for position, version 1,2
    pub nac_p: u8,

    #[deku(bits = "2")]
    #[serde(rename = "SIL")]
    /// Surveillance Integrity Level (SIL)
    pub sil: u8,

    #[deku(bits = "1")]
    #[serde(rename = "TAH")]
    pub track_angle_or_heading: u8,

    #[deku(bits = "1")]
    #[deku(pad_bits_after = "2")]
    #[serde(rename = "HRD")]
    // 1 for magnetic, 0 for true north
    pub horizontal_reference_direction: u8,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct SurfaceV2 {
    #[deku(bits = "1")]
    #[serde(rename = "NICa")]
    /// NIC suppelement A (NICa)
    pub nic_a: u8,

    #[deku(bits = "4")]
    #[deku(pad_bits_after = "2")]
    #[serde(rename = "NACp")]
    /// Navigation Accuracy Category for position (NACp)
    pub nac_p: u8,

    #[deku(bits = "2")]
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
    /// SIL supplement bit, only in version 2:
    /// 0 means per hour,
    /// 1 means per sample.
    pub sil_supplement: u8,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct Empty {}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct EmptyU8 {
    pub id: u8,
    pub unused: u8,
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Copy, Clone)]
pub struct ReservedStatus {
    pub data: [u8; 5],
}
