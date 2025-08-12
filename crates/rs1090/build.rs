#[cfg(feature = "sero")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .compile_protos(&["proto/SeRoAPI.proto"], &["proto"])?;
    Ok(())
}
#[cfg(not(feature = "sero"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
