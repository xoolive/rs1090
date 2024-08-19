use std::str::FromStr;

use radarcape::Address;
use rs1090::decode::{cpr::Position, TimedMessage};
use rs1090::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tracing::error;
use url::Url;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Protocol {
    #[default]
    Tcp,
    Udp,
    Websocket(String),
    Rtlsdr,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Source {
    scheme: Protocol,
    host: String,
    port: u16,
    pub airport: Option<String>,
    pub reference: Option<Position>,
    #[serde(skip)]
    pub count: u64,
    #[serde(skip)]
    pub last: u64,
}

impl FromStr for Source {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut source = Source::default();

        let s = s.replace("@", "?"); // retro-compatibility
        let default_tcp = Url::parse("tcp://").unwrap();

        let url = default_tcp.join(&s).map_err(|e| e.to_string())?;

        source.scheme = match url.scheme() {
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            "rtlsdr" => Protocol::Rtlsdr,
            "ws" => Protocol::Websocket(
                url.path().strip_prefix("/").unwrap().to_string(),
            ),
            _ => return Err("unsupported scheme".to_string()),
        };

        if source.scheme != Protocol::Rtlsdr {
            match url.host_str() {
                Some(host) => {
                    source.host = host.to_owned();
                    source.port = url.port_or_known_default().unwrap_or(10003);
                }
                None => {
                    // particular cornercase if we only enter the port number
                    source.host = "0.0.0.0".to_owned();
                    source.port = url
                        .path()
                        .strip_prefix("/:")
                        .unwrap()
                        .parse::<u16>()
                        .expect("A port number was expected");
                }
            }
        }

        if let Some(query) = url.query() {
            if !query.contains(',') {
                source.airport = Some(query.to_string());
            }
            source.reference = Position::from_str(query).ok()
        };

        Ok(source)
    }
}

impl Source {
    pub async fn receiver(&self, tx: Sender<TimedMessage>, idx: usize) {
        if self.scheme == Protocol::Rtlsdr {
            #[cfg(not(feature = "rtlsdr"))]
            {
                eprintln!(
                    "Not compiled with RTL-SDR support, use the rtlsdr feature"
                );
                std::process::exit(127);
            }
            #[cfg(feature = "rtlsdr")]
            {
                rtlsdr::discover();
                rtlsdr::receiver(tx, idx).await
            }
        } else {
            let server_address = match &self.scheme {
                Protocol::Tcp => {
                    Address::TCP(format!("{}:{}", self.host, self.port))
                }
                Protocol::Udp => {
                    Address::UDP(format!("{}:{}", self.host, self.port))
                }
                Protocol::Websocket(path) => Address::Websocket(format!(
                    "ws://{}:{}/{}",
                    self.host, self.port, path
                )),
                Protocol::Rtlsdr => unreachable!(),
            };
            if let Err(e) = radarcape::receiver(server_address, tx, idx).await {
                error!("{}", e.to_string());
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
        if let Ok(Source { scheme, .. }) = source {
            assert_eq!(scheme, Protocol::Rtlsdr);
        }

        let source = Source::from_str("rtlsdr:@LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            scheme,
            airport,
            reference: Some(pos),
            ..
        }) = source
        {
            assert_eq!(scheme, Protocol::Rtlsdr);
            assert_eq!(airport, Some("LFBO".to_string()));
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }

        let source = Source::from_str("http://default");
        assert!(source.is_err());

        let source = Source::from_str(":4003");
        assert!(source.is_ok());
        if let Ok(Source {
            scheme,
            host,
            port,
            airport,
            reference,
            ..
        }) = source
        {
            assert_eq!(scheme, Protocol::Tcp);
            assert_eq!(host, "0.0.0.0");
            assert_eq!(port, 4003);
            assert_eq!(airport, None);
            assert_eq!(reference, None);
        }

        let source = Source::from_str(":4003?LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            scheme,
            host,
            port,
            airport,
            reference: Some(pos),
            ..
        }) = source
        {
            assert_eq!(scheme, Protocol::Tcp);
            assert_eq!(host, "0.0.0.0");
            assert_eq!(port, 4003);
            assert_eq!(airport, Some("LFBO".to_string()));
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }

        let source = Source::from_str("ws://1.2.3.4:4003/get?LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            scheme,
            host,
            port,
            airport,
            reference: Some(pos),
            ..
        }) = source
        {
            assert_eq!(scheme, Protocol::Websocket("get".to_string()));
            assert_eq!(host, "1.2.3.4");
            assert_eq!(port, 4003);
            assert_eq!(airport, Some("LFBO".to_string()));
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }
    }
}
