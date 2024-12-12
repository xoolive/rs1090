#[cfg(feature = "sero")]
use rs1090::source::sero::SeroClient;

#[tokio::main]
#[cfg(feature = "sero")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load TOKEN from environment variables
    let client = SeroClient {
        token: std::env::var("SERO_TOKEN")?,
        df_filter: vec![],
        aircraft_filter: vec![],
    };

    // Access info about receivers and display it
    let info = client.info().await.unwrap();
    for sensor in info.sensor_info {
        let gnss = sensor.gnss.unwrap();
        println!(
            "{} {} {} {} {}",
            sensor.sensor.unwrap().serial,
            gnss.position.unwrap().latitude,
            gnss.position.unwrap().longitude,
            gnss.position.unwrap().height,
            sensor.alias,
        );
    }

    Ok(())
}

#[cfg(not(feature = "sero"))]
fn main() {}
