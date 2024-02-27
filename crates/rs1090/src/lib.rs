#![doc = include_str!("../readme.md")]
pub mod decode;
pub mod source;

pub mod prelude {
    /// This re-export is necessary to decode messages
    pub use deku::prelude::*;

    pub use crate::decode::adsb::ME::*;
    pub use crate::decode::bds::bds05::AirbornePosition;
    pub use crate::decode::bds::bds06::SurfacePosition;
    pub use crate::decode::bds::bds08::AircraftIdentification;
    pub use crate::decode::bds::bds09::AirborneVelocity;
    pub use crate::decode::bds::bds61::AircraftStatus;
    pub use crate::decode::bds::bds62::TargetStateAndStatusInformation;
    pub use crate::decode::bds::bds65::AircraftOperationStatus;
    /// The root structure to decode messages
    pub use crate::decode::Message;
    pub use crate::decode::DF::*;

    /// This re-export is necessary for the following export
    pub use futures_util::stream::StreamExt;

    /// Information on the structure of a Beast message
    pub use crate::source::beast;
}
