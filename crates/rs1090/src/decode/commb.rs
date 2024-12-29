use super::bds::bds05::AirbornePosition;
use super::bds::bds10::DataLinkCapability;
use super::bds::bds17::CommonUsageGICBCapabilityReport;
use super::bds::bds18::GICBCapabilityReportPart1;
use super::bds::bds19::GICBCapabilityReportPart2;
use super::bds::bds20::AircraftIdentification;
use super::bds::bds21::AircraftAndAirlineRegistrationMarkings;
use super::bds::bds30::ACASResolutionAdvisory;
use super::bds::bds40::SelectedVerticalIntention;
use super::bds::bds44::MeteorologicalRoutineAirReport;
use super::bds::bds45::MeteorologicalHazardReport;
use super::bds::bds50::TrackAndTurnReport;
use super::bds::bds60::HeadingAndSpeedReport;
use super::bds::bds65::AircraftOperationStatus;
use super::AC13Field;
use deku::prelude::*;
use serde::Serialize;
use std::fmt;
use tracing::debug;

/**
 * ## Comm-B Data Selector (BDS)
 *
 * The first four BDS codes (1,0, 1,7, 2,0, 3,0) belong to the ELS service,
 * the next three ones (4,0, 5,0, 6,0) belong to the EHS services,
 * and the last two codes (4,4, 4,5) report meteorological information.
 */

#[derive(Debug, PartialEq, Serialize, Clone, Default)]
pub struct DF20DataSelector {
    #[serde(skip)]
    /// Set to true if all zeros, then there is no need to parse
    pub is_empty: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds05: Option<AirbornePosition>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds10: Option<DataLinkCapability>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds17: Option<CommonUsageGICBCapabilityReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds18: Option<GICBCapabilityReportPart1>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds19: Option<GICBCapabilityReportPart2>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds20: Option<AircraftIdentification>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds21: Option<AircraftAndAirlineRegistrationMarkings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds30: Option<ACASResolutionAdvisory>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds40: Option<SelectedVerticalIntention>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds44: Option<MeteorologicalRoutineAirReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds45: Option<MeteorologicalHazardReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds50: Option<TrackAndTurnReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds60: Option<HeadingAndSpeedReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds65: Option<AircraftOperationStatus>,
}

#[derive(Debug, PartialEq, Serialize, Clone, Default)]
pub struct DF21DataSelector {
    #[serde(skip)]
    /// Set to true if all zeros, then there is no need to parse
    pub is_empty: bool,

    // On purpose: do not try bds05 here.
    // The reason for that is that there is no way to validate the altitude
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds10: Option<DataLinkCapability>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds17: Option<CommonUsageGICBCapabilityReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds18: Option<GICBCapabilityReportPart1>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds19: Option<GICBCapabilityReportPart2>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds20: Option<AircraftIdentification>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds21: Option<AircraftAndAirlineRegistrationMarkings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds30: Option<ACASResolutionAdvisory>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds40: Option<SelectedVerticalIntention>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds44: Option<MeteorologicalRoutineAirReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds45: Option<MeteorologicalHazardReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds50: Option<TrackAndTurnReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds60: Option<HeadingAndSpeedReport>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bds65: Option<AircraftOperationStatus>,
}

impl fmt::Display for DF21DataSelector {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl fmt::Display for DF20DataSelector {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl DekuReader<'_, AC13Field> for DF20DataSelector {
    fn from_reader_with_ctx<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
        reader: &mut Reader<R>,
        ac: AC13Field, // altitude helps a lot in the validation
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let mut result = Self::default();
        let res = reader.read_bits(56)?;
        let bits = res.unwrap();
        let buf = bits.into_vec();
        debug!(
            "Decoding {:?} according to various hypotheses",
            buf.as_slice()
        );

        if buf.iter().all(|&x| x == 0) {
            result.is_empty = true;
            return Ok(result);
        }

        // Read the first 5 bits as a u8 and get the typecode
        let mut cursor = deku::no_std_io::Cursor::new(buf.as_slice());
        let tc_reader = &mut Reader::new(&mut cursor);
        let tc = u8::from_reader_with_ctx(tc_reader, deku::ctx::BitSize(5))?;

        if (9..22).contains(&tc) && tc != 19 {
            match AirbornePosition::try_from(buf.as_slice()) {
                Ok(bds05) => match bds05.alt {
                    Some(alt) if alt == ac.0 => result.bds05 = Some(bds05),
                    _ => (),
                },
                Err(e) => debug!("Hypothesis BDS05: {}", e.to_string()),
            }
        } else {
            debug!(
                "Hypothesis BDS05: Typecode inconsistency {} should be in [9, 18] or [20, 22]",
                tc
            )
        }
        match DataLinkCapability::try_from(buf.as_slice()) {
            Ok(bds10) => result.bds10 = Some(bds10),
            Err(e) => debug!("Hypothesis BDS10: {}", e.to_string()),
        }
        match CommonUsageGICBCapabilityReport::try_from(buf.as_slice()) {
            Ok(bds17) => result.bds17 = Some(bds17),
            Err(e) => debug!("Hypothesis BDS17: {}", e.to_string()),
        }
        match GICBCapabilityReportPart1::try_from(buf.as_slice()) {
            Ok(bds18) => result.bds18 = Some(bds18),
            Err(e) => debug!("Hypothesis BDS18: {}", e.to_string()),
        }
        match GICBCapabilityReportPart2::try_from(buf.as_slice()) {
            Ok(bds19) => result.bds19 = Some(bds19),
            Err(e) => debug!("Hypothesis BDS19: {}", e.to_string()),
        }
        match AircraftIdentification::try_from(buf.as_slice()) {
            Ok(bds20) => result.bds20 = Some(bds20),
            Err(e) => debug!("Hypothesis BDS20: {}", e.to_string()),
        }
        match AircraftAndAirlineRegistrationMarkings::try_from(buf.as_slice()) {
            Ok(bds21) => result.bds21 = Some(bds21),
            Err(e) => debug!("Error BDS21: {}", e.to_string()),
        }
        match ACASResolutionAdvisory::try_from(buf.as_slice()) {
            Ok(bds30) => result.bds30 = Some(bds30),
            Err(e) => debug!("Hypothesis BDS30: {}", e.to_string()),
        }
        match SelectedVerticalIntention::try_from(buf.as_slice()) {
            Ok(bds40) => result.bds40 = Some(bds40),
            Err(e) => debug!("Hypothesis BDS40: {}", e.to_string()),
        }
        match MeteorologicalRoutineAirReport::try_from(buf.as_slice()) {
            Ok(bds44) => result.bds44 = Some(bds44),
            Err(e) => debug!("Hypothesis BDS44: {}", e.to_string()),
        }
        match MeteorologicalHazardReport::try_from(buf.as_slice()) {
            Ok(bds45) => result.bds45 = Some(bds45),
            Err(e) => debug!("Hypothesis BDS45: {}", e.to_string()),
        }
        match TrackAndTurnReport::try_from(buf.as_slice()) {
            Ok(bds50) => result.bds50 = Some(bds50),
            Err(e) => debug!("Hypothesis BDS50: {}", e.to_string()),
        }
        match HeadingAndSpeedReport::try_from(buf.as_slice()) {
            Ok(bds60) => result.bds60 = Some(bds60),
            Err(e) => debug!("Hypothesis BDS60: {}", e.to_string()),
        }

        let mut cursor = deku::no_std_io::Cursor::new(buf.as_slice());
        let bds65_reader = &mut Reader::new(&mut cursor);
        let tc = u8::from_reader_with_ctx(bds65_reader, deku::ctx::BitSize(5))?;

        if tc == 31 {
            let enum_id =
                u8::from_reader_with_ctx(bds65_reader, deku::ctx::BitSize(3))?;
            match (enum_id, AircraftOperationStatus::try_from(buf.as_slice())) {
                (_, Err(e)) => debug!("Hypothesis BDS65: {}", e.to_string()),
                (id, _) if id >= 2 => {
                    debug!("Hypothesis BDS65: Reserved field: id={}", id)
                }
                (_, Ok(bds65)) => result.bds65 = Some(bds65),
            }
        } else {
            debug!("Hypothesis BDS65: Typecode {} should be 31", tc)
        }

        Ok(result)
    }
}

impl DekuReader<'_> for DF21DataSelector {
    fn from_reader_with_ctx<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
        reader: &mut Reader<R>,
        _: (),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let mut result = Self::default();
        let res = reader.read_bits(56)?;
        let buf = res.unwrap().into_vec();
        debug!(
            "Decoding {:?} according to various hypotheses",
            buf.as_slice()
        );

        if buf.iter().all(|&x| x == 0) {
            result.is_empty = true;
            return Ok(result);
        }
        // Do not attempt to decode BDS0,5:
        // DF21 does not contain any altitude to validate the info
        /*match AirbornePosition::try_from(buf.as_slice()) {
            Ok(bds05) => match bds05.alt {
                None => Ok(None),
                Some(alt) if alt == ac.0 => Ok(Some(bds05)),
                Some(_) => Ok(None),
            },
            Err(e) => debug!("Error BDS05: {}", e.to_string()),
        }*/
        match DataLinkCapability::try_from(buf.as_slice()) {
            Ok(bds10) => result.bds10 = Some(bds10),
            Err(e) => debug!("Error BDS10: {}", e.to_string()),
        }
        match CommonUsageGICBCapabilityReport::try_from(buf.as_slice()) {
            Ok(bds17) => result.bds17 = Some(bds17),
            Err(e) => debug!("Error BDS17: {}", e.to_string()),
        }
        match GICBCapabilityReportPart1::try_from(buf.as_slice()) {
            Ok(bds18) => result.bds18 = Some(bds18),
            Err(e) => debug!("Error BDS18: {}", e.to_string()),
        }
        match GICBCapabilityReportPart2::try_from(buf.as_slice()) {
            Ok(bds19) => result.bds19 = Some(bds19),
            Err(e) => debug!("Error BDS19: {}", e.to_string()),
        }
        match AircraftIdentification::try_from(buf.as_slice()) {
            Ok(bds20) => result.bds20 = Some(bds20),
            Err(e) => debug!("Error BDS20: {}", e.to_string()),
        }
        match AircraftAndAirlineRegistrationMarkings::try_from(buf.as_slice()) {
            Ok(bds21) => result.bds21 = Some(bds21),
            Err(e) => debug!("Error BDS21: {}", e.to_string()),
        }
        match ACASResolutionAdvisory::try_from(buf.as_slice()) {
            Ok(bds30) => result.bds30 = Some(bds30),
            Err(e) => debug!("Error BDS30: {}", e.to_string()),
        }
        match SelectedVerticalIntention::try_from(buf.as_slice()) {
            Ok(bds40) => result.bds40 = Some(bds40),
            Err(e) => debug!("Error BDS40: {}", e.to_string()),
        }
        match MeteorologicalRoutineAirReport::try_from(buf.as_slice()) {
            Ok(bds44) => result.bds44 = Some(bds44),
            Err(e) => debug!("Error BDS44: {}", e.to_string()),
        }
        match MeteorologicalHazardReport::try_from(buf.as_slice()) {
            Ok(bds45) => result.bds45 = Some(bds45),
            Err(e) => debug!("Error BDS45: {}", e.to_string()),
        }
        match TrackAndTurnReport::try_from(buf.as_slice()) {
            Ok(bds50) => result.bds50 = Some(bds50),
            Err(e) => debug!("Error BDS50: {}", e.to_string()),
        }
        match HeadingAndSpeedReport::try_from(buf.as_slice()) {
            Ok(bds60) => result.bds60 = Some(bds60),
            Err(e) => debug!("Error BDS60: {}", e.to_string()),
        }

        let mut cursor = deku::no_std_io::Cursor::new(buf.as_slice());
        let bds65_reader = &mut Reader::new(&mut cursor);
        let tc = u8::from_reader_with_ctx(bds65_reader, deku::ctx::BitSize(5))?;

        if tc == 31 {
            let enum_id =
                u8::from_reader_with_ctx(bds65_reader, deku::ctx::BitSize(3))?;
            match (enum_id, AircraftOperationStatus::try_from(buf.as_slice())) {
                (_, Err(e)) => debug!("Hypothesis BDS65: {}", e.to_string()),
                (id, _) if id >= 2 => {
                    debug!("Hypothesis BDS65: Reserved field: id={}", id)
                }
                (_, Ok(bds65)) => result.bds65 = Some(bds65),
            }
        } else {
            debug!("Hypothesis BDS65: Typecode {} should be 31", tc)
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use hexlit::hex;

    #[test]
    fn test_bds5060_no65() {
        let bytes = hex!("A8001EBCFFFB23286004A73F6A5B");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        match msg.df {
            CommBIdentityReply { bds, .. } => {
                assert!(bds.bds50.is_some());
                assert!(bds.bds60.is_some());
                assert!(bds.bds65.is_none());
            }
            _ => unreachable!(),
        }
    }
}
