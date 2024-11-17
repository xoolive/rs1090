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
use tokio::fs;
use tonic::{
    transport::{Certificate, Channel, ClientTlsConfig},
    Streaming,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeroClient {
    pub token: String,
    pub ca_cert: String,
}

impl SeroClient {
    pub async fn client(
        self,
    ) -> Result<SeRoApiClient<Channel>, Box<dyn std::error::Error>> {
        // Load the CA certificate
        let ca_cert = fs::read(self.ca_cert).await?;
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

    pub async fn info(self) -> Option<SensorInfoResponse> {
        let request = tonic::Request::new(SensorInfoRequest {
            token: self.token.clone(),
            sensors: vec![],
        });
        if let Some(mut client) = self.client().await.ok() {
            client
                .get_sensor_info(request)
                .await
                .ok()
                .map(|s| s.into_inner())
        } else {
            None
        }
    }

    pub async fn rawstream(self) -> Option<Streaming<ModeSDownlinkFrame>> {
        let request = tonic::Request::new(ModeSDownlinkFramesRequest {
            token: self.token.clone(),
            df_filter: vec![],
            sensor_filter: vec![],
            aircraft_filter: vec![],
        });
        // This may look weird but causes error if we write if let Ok()
        // This is due to dyn StdError not being thread-safe
        if let Some(mut client) = self.client().await.ok() {
            client
                .get_mode_s_downlink_frames(request)
                .await
                .ok()
                .map(|s| s.into_inner())
        } else {
            None
        }
    }
}
