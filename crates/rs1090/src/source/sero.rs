#![allow(clippy::match_result_ok)]
mod api {
    tonic::include_proto!("serosystems.proto.v3.backend.api");
}

use api::{
    se_ro_api_client::SeRoApiClient, ModeSDownlinkFrame,
    ModeSDownlinkFramesRequest, SensorInfoRequest, SensorInfoResponse,
};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tonic::{
    transport::{Certificate, Channel, ClientTlsConfig},
    Streaming,
};
use tracing::{error, info};

use crate::decode::time::now_in_ns;
use crate::decode::time::since_gps_week_to_since_today;
use crate::decode::time::since_gps_week_to_unix_s;
use crate::prelude::*;

type Result<T> =
    std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeroClient {
    pub token: String,
    pub df_filter: Vec<u32>,
    pub aircraft_filter: Vec<u32>,
}

async fn download_file(url: &str, destination: &PathBuf) -> Result<()> {
    let response = reqwest::get(url).await?.bytes().await?;
    let mut file = File::create(destination).await?;
    file.write_all(&response).await?;
    Ok(())
}

pub async fn receiver(sero: SeroClient, tx: mpsc::Sender<TimedMessage>) {
    let mut stream = sero.rawstream().await.unwrap();
    let tx_copy = tx.clone();
    let info = &sero.info().await.unwrap();
    let sensor_map: HashMap<u64, String> = info
        .sensor_info
        .iter()
        .map(|elt| (elt.sensor.unwrap().serial, elt.alias.to_string()))
        .collect();
    tokio::spawn(async move {
        while let Some(response) = stream.next().await {
            if let Ok(msg) = response {
                let bytes = msg.reply.as_slice();
                let system_timestamp = now_in_ns() as f64 * 1e-9;
                let metadata = msg
                    .receptions
                    .into_iter()
                    .map(|rm| SensorMetadata {
                        system_timestamp,
                        gnss_timestamp: Some(since_gps_week_to_unix_s(
                            rm.gnss_timestamp,
                        )),
                        nanoseconds: Some(since_gps_week_to_since_today(
                            rm.gnss_timestamp,
                        )),
                        rssi: Some(rm.signal_level),
                        serial: rm.sensor.unwrap().serial,
                        name: sensor_map
                            .get(&rm.sensor.unwrap().serial)
                            .cloned(),
                    })
                    .collect();

                let tmsg = TimedMessage {
                    timestamp: system_timestamp,
                    frame: bytes.to_vec(),
                    message: None,
                    metadata,
                    decode_time: None,
                };
                if let Err(e) = tx_copy.send(tmsg).await {
                    error!("{}", e.to_string());
                }
            }
        }
    });
}

impl SeroClient {
    pub async fn client(&self) -> Result<SeRoApiClient<Channel>> {
        let mut cache_path = dirs::cache_dir().unwrap_or_default();
        cache_path.push("jet1090");
        if !cache_path.exists() {
            let msg =
                format!("failed to create {:?}", cache_path.to_str().unwrap());
            fs::create_dir_all(&cache_path).await.expect(&msg);
        }

        let ca_cert_url = "https://doc.sero-systems.de/api/_downloads/017ce4f89360621e0345c257b6136b21/ca.crt";
        let ca_cert_file = "ca_sero.crt";

        cache_path.push(ca_cert_file);
        if !cache_path.exists() {
            info!("Downloading sero certificate");
            download_file(ca_cert_url, &cache_path).await?;
        }

        // Load the CA certificate
        info!("Loading sero certificate: {:?}", cache_path);
        let ca_cert = fs::read(cache_path).await?;
        let ca_cert = Certificate::from_pem(ca_cert);

        // Configure TLS
        let tls_config = ClientTlsConfig::new().ca_certificate(ca_cert);

        // Build the channel with TLS configuration
        let channel = Channel::from_static("https://api.secureadsb.com:4201")
            .tls_config(tls_config)?
            .connect()
            .await?;

        Ok(SeRoApiClient::new(channel))
    }

    pub async fn info(&self) -> Result<SensorInfoResponse> {
        let request = tonic::Request::new(SensorInfoRequest {
            token: self.token.clone(),
            sensors: vec![],
        });
        Ok(self
            .client()
            .await?
            .get_sensor_info(request)
            .await?
            .into_inner())
    }

    pub async fn rawstream(&self) -> Result<Streaming<ModeSDownlinkFrame>> {
        let request = tonic::Request::new(ModeSDownlinkFramesRequest {
            token: self.token.clone(),
            df_filter: self.df_filter.clone(),
            sensor_filter: vec![],
            aircraft_filter: self.aircraft_filter.clone(),
        });
        Ok(self
            .client()
            .await?
            .get_mode_s_downlink_frames(request)
            .await?
            .into_inner())
    }
}
