use std::str::FromStr;

use radarcape::BeastSource;
use rs1090::decode::{cpr::Position, TimedMessage};
use rs1090::prelude::*;
#[cfg(feature = "sero")]
use rs1090::source::sero::SeroClient;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tracing::error;
use url::Url;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Address {
    Tcp(String),
    Udp(String),
    Websocket(String),
    Rtlsdr(Option<String>),
    #[cfg(feature = "sero")]
    Sero(SeroClient),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    address: Address,
    pub name: Option<String>,
    pub reference: Option<Position>,
    #[serde(skip)]
    pub count: u64,
    #[serde(skip)]
    pub last: u64,
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
            count: 0,
            last: 0,
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
        if let Address::Rtlsdr(args) = &self.address {
            #[cfg(not(feature = "rtlsdr"))]
            {
                error!(
                    "rtlsdr://{} ignored",
                    args.clone().unwrap_or("".to_string())
                );
                eprintln!("Compile jet1090 with the rtlsdr feature");
                std::process::exit(127);
            }
            #[cfg(feature = "rtlsdr")]
            {
                rtlsdr::receiver::<&str>(tx, args.as_deref(), serial, name)
                    .await
            }
        } else {
            #[cfg(feature = "sero")]
            {
                if let Address::Sero(sero) = self.address.clone() {
                    let mut stream = sero.rawstream().await.unwrap();
                    let tx_copy = tx.clone();
                    tokio::spawn(async move {
                        while let Some(response) = stream.next().await {
                            if let Ok(msg) = response {
                                let bytes = msg.reply.as_slice();
                                if let Err(e) = tx_copy
                                    .send(TimedMessage {
                                        timestamp: msg.receptions[0]
                                            .sensor_timestamp
                                            as f64
                                            * 1e-3,
                                        timesource: rs1090::decode::TimeSource::Radarcape,
                                        rssi: None,
                                        frame: hex::encode(bytes),
                                        message: None,
                                        idx: 0,
                                    })
                                    .await
                                {
                                    error!("{}", e.to_string());
                                }
                            }
                        }
                    });
                    return;
                }
            }
            let server_address = match &self.address {
                Address::Tcp(s) => BeastSource::TCP(s.to_owned()),
                Address::Udp(s) => BeastSource::UDP(s.to_owned()),
                Address::Websocket(s) => BeastSource::Websocket(s.to_owned()),
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
