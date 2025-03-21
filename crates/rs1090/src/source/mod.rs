#[cfg(not(target_arch = "wasm32"))]
pub mod beast;

#[cfg(feature = "rtlsdr")]
pub mod rtlsdr;

#[cfg(feature = "sero")]
pub mod sero;

#[cfg(feature = "ssh")]
pub mod sshjump;
#[cfg(feature = "ssh")]
pub mod sshtunnel;
