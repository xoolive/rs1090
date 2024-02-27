#![doc = include_str!("../readme.md")]
pub mod decode;
pub mod source;

pub mod prelude {
    /// This re-export is necessary to decode messages
    pub use deku::prelude::*;

    /// The root structure to decode messages
    pub use crate::decode::Message;

    /// This re-export is necessary for the following export
    pub use futures_util::stream::StreamExt;

    /// Information on the structure of a Beast message
    pub use crate::source::beast;
}
