#[cfg(not(target_arch = "wasm32"))]
pub mod beast;

pub mod demod;

#[cfg(feature = "rtlsdr")]
pub mod rtlsdr;

#[cfg(feature = "sero")]
pub mod sero;

#[cfg(feature = "soapy")]
pub mod soapy;

#[cfg(feature = "ssh")]
pub mod ssh;
