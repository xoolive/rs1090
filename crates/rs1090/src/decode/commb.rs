use super::bds::bds10::DataLinkCapability;
use super::bds::bds17::GICBCapabilityReport;
use super::bds::bds20::AircraftIdentification;
use super::bds::bds30::ACASResolutionAdvisory;
use super::bds::bds40::SelectedVerticalIntention;
use super::bds::bds44::MeteorologicalRoutineAirReport;
use super::bds::bds50::TrackAndTurnReport;
use super::bds::bds60::HeadingAndSpeedReport;
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
pub struct DataSelector {
    #[serde(skip)]
    /// Set to true if all zeros, then there is no need to parse
    pub is_empty: bool,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub bds10: Option<DataLinkCapability>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub bds17: Option<GICBCapabilityReport>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub bds20: Option<AircraftIdentification>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub bds30: Option<ACASResolutionAdvisory>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub bds40: Option<SelectedVerticalIntention>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub bds44: Option<MeteorologicalRoutineAirReport>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub bds50: Option<TrackAndTurnReport>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub bds60: Option<HeadingAndSpeedReport>,
}

impl fmt::Display for DataSelector {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl DekuReader<'_> for DataSelector {
    fn from_reader_with_ctx<R: deku::no_std_io::Read>(
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
        match DataLinkCapability::try_from(buf.as_slice()) {
            Ok(bds10) => result.bds10 = Some(bds10),
            Err(e) => debug!("Error BDS10: {}", e.to_string()),
        }
        match GICBCapabilityReport::try_from(buf.as_slice()) {
            Ok(bds17) => result.bds17 = Some(bds17),
            Err(e) => debug!("Error BDS17: {}", e.to_string()),
        }
        match AircraftIdentification::try_from(buf.as_slice()) {
            Ok(bds20) => result.bds20 = Some(bds20),
            Err(e) => debug!("Error BDS20: {}", e.to_string()),
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
        match TrackAndTurnReport::try_from(buf.as_slice()) {
            Ok(bds50) => result.bds50 = Some(bds50),
            Err(e) => debug!("Error BDS50: {}", e.to_string()),
        }
        match HeadingAndSpeedReport::try_from(buf.as_slice()) {
            Ok(bds60) => result.bds60 = Some(bds60),
            Err(e) => debug!("Error BDS60: {}", e.to_string()),
        }
        Ok(result)
    }
}
