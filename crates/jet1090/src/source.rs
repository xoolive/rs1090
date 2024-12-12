use radarcape::BeastSource;
use rs1090::prelude::*;
#[cfg(feature = "rtlsdr")]
use rs1090::source::rtlsdr;
#[cfg(feature = "sero")]
use rs1090::source::sero;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use tokio::sync::mpsc::Sender;
use tracing::error;
use url::Url;

// An intermediate structure defined so that you can keep your Sero entries in
// your configuration file even if the sero feature is not activated
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeroParams {
    pub token: String,
    pub df_filter: Option<Vec<u32>>,
    pub aircraft_filter: Option<Vec<u32>>,
}

#[cfg(feature = "sero")]
impl From<&SeroParams> for sero::SeroClient {
    fn from(value: &SeroParams) -> Self {
        sero::SeroClient {
            token: value.token.clone(),
            df_filter: value.df_filter.clone().unwrap_or_default(),
            aircraft_filter: value.aircraft_filter.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Address {
    Tcp(String),
    Udp(String),
    Websocket(String),
    Rtlsdr(Option<String>),
    Sero(SeroParams),
}

fn build_serial(input: &str) -> u64 {
    // Create a hasher
    let mut hasher = DefaultHasher::new();
    // Hash the string
    input.hash(&mut hasher);
    // Get the hash as a u64
    hasher.finish()
}

impl Source {
    pub fn serial(&self) -> u64 {
        match &self.address {
            Address::Tcp(name) => build_serial(name),
            Address::Udp(name) => build_serial(name),
            Address::Websocket(name) => build_serial(name),
            Address::Rtlsdr(reference) => {
                let name = reference.clone().unwrap_or("rtlsdr".to_string());
                build_serial(&name)
            }
            Address::Sero(_) => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub address: Address,
    pub name: Option<String>,
    pub reference: Option<Position>,
}

impl FromStr for Source {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.replace("@", "?"); // retro-compatibility
        let default_tcp = Url::parse("tcp://").unwrap();

        let url = default_tcp.join(&s).map_err(|e| e.to_string())?;

        let address = match url.scheme() {
            "tcp" => Address::Tcp(format!(
                "{}:{}",
                url.host_str().unwrap_or("0.0.0.0"),
                match url.host() {
                    Some(_) => url.port_or_known_default().unwrap_or(10003),
                    None => {
                        // deals with ":4003?LFBO" (parsed as "tcp:///:4003?LFBO")
                        url.path()
                            .strip_prefix("/:")
                            .unwrap()
                            .parse::<u16>()
                            .expect("A port number was expected")
                    }
                }
            )),
            "udp" => Address::Udp(format!(
                "{}:{}",
                url.host_str().unwrap_or("0.0.0.0"),
                url.port_or_known_default().unwrap()
            )),
            "rtlsdr" => Address::Rtlsdr(url.host_str().map(|s| s.to_string())),
            "ws" => Address::Websocket(format!(
                "ws://{}:{}/{}",
                url.host_str().unwrap_or("0.0.0.0"),
                url.port_or_known_default().unwrap(),
                url.path().strip_prefix("/").unwrap()
            )),
            _ => return Err("unsupported scheme".to_string()),
        };

        let mut source = Source {
            address,
            name: None,
            reference: None,
        };

        if let Some(query) = url.query() {
            source.reference = Position::from_str(query).ok()
        };

        Ok(source)
    }
}

impl Source {
    pub async fn receiver(
        &self,
        tx: Sender<TimedMessage>,
        serial: u64,
        name: Option<String>,
    ) {
        match &self.address {
            Address::Rtlsdr(args) => {
                #[cfg(not(feature = "rtlsdr"))]
                {
                    error!("Compile jet1090 with the rtlsdr feature, {:?} argument ignored", args);
                    std::process::exit(127);
                }
                #[cfg(feature = "rtlsdr")]
                {
                    rtlsdr::receiver::<&str>(tx, args.as_deref(), serial, name)
                        .await
                }
            }
            Address::Sero(sero) => {
                #[cfg(not(feature = "sero"))]
                {
                    error!("Compile jet1090 with the sero feature, {:?} argument ignored", sero);
                }
                #[cfg(feature = "sero")]
                {
                    sero::receiver(sero::SeroClient::from(sero), tx).await
                }
            }
            _ => {
                let server_address = match &self.address {
                    Address::Tcp(s) => BeastSource::TCP(s.to_owned()),
                    Address::Udp(s) => BeastSource::UDP(s.to_owned()),
                    Address::Websocket(s) => {
                        BeastSource::Websocket(s.to_owned())
                    }
                    _ => unreachable!(),
                };
                if let Err(e) =
                    radarcape::receiver(server_address, tx, serial, name).await
                {
                    error!("{}", e.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_source() {
        let source = Source::from_str("rtlsdr:");
        assert!(source.is_ok());
        if let Ok(Source { address, .. }) = source {
            assert_eq!(address, Address::Rtlsdr(None));
        }

        let source = Source::from_str("rtlsdr://serial=00000001");
        assert!(source.is_ok());
        if let Ok(Source { address, .. }) = source {
            assert_eq!(
                address,
                Address::Rtlsdr(Some("serial=00000001".to_string()))
            );
        }

        let source = Source::from_str("rtlsdr:@LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            address,
            name,
            reference: Some(pos),
            ..
        }) = source
        {
            assert_eq!(address, Address::Rtlsdr(None));
            assert_eq!(name, None);
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }

        let source = Source::from_str("http://default");
        assert!(source.is_err());

        let source = Source::from_str(":4003");
        assert!(source.is_ok());
        if let Ok(Source {
            address: Address::Tcp(path),
            name,
            reference,
            ..
        }) = source
        {
            assert_eq!(path, "0.0.0.0:4003");
            assert_eq!(name, None);
            assert_eq!(reference, None);
        }

        let source = Source::from_str(":4003?LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            address: Address::Tcp(path),
            name,
            reference: Some(pos),
            ..
        }) = source
        {
            assert_eq!(path, "0.0.0.0:4003");
            assert_eq!(name, None);
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }

        let source = Source::from_str("ws://1.2.3.4:4003/get?LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            address,
            name,
            reference: Some(pos),
            ..
        }) = source
        {
            assert_eq!(
                address,
                Address::Websocket("ws://1.2.3.4:4003/get".to_string())
            );
            assert_eq!(name, None);
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }
    }
}
