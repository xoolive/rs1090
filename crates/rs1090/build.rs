#[cfg(feature = "sero")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/SeRoAPI.proto")?;
    Ok(())
}
#[cfg(not(feature = "sero"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
