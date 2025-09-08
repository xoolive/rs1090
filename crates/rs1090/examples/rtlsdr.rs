#[cfg(feature = "soapy")]
use soapysdr::{enumerate, Device};
use tracing::info;

#[cfg(feature = "soapy")]
fn main() {
    let device = enumerate("driver=rtlsdr").unwrap();
    for arg in device {
        println!("{arg:#}");

        let device = match Device::new(arg) {
            Ok(device) => {
                info!("{:#}", device.hardware_info().unwrap());
                device
            }
            Err(error) => {
                eprintln!("SoapySDR error: {error}");
                std::process::exit(127);
            }
        };
        println!("{:#}", device.hardware_info().unwrap());
    }
}

#[cfg(not(feature = "soapy"))]
fn main() {
    info!("rtlsdr feature not activated");
}
