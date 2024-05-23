use std::str::FromStr;

use rs1090::decode::{cpr::Position, TimedMessage};
use rs1090::prelude::*;
use serde::Serialize;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Source {
    host: String,
    port: u16,
    rtlsdr: bool,
    pub airport: Option<String>,
    pub reference: Option<Position>,
    pub count: u64,
    pub last: u64,
}

impl FromStr for Source {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('@').map(|p| p.trim()).collect();

        let mut source = Source::default();
        if parts.len() == 2 {
            if !parts[1].contains(',') {
                source.airport = Some(parts[1].to_string());
            }
            source.reference = Position::from_str(parts[1]).ok()
        };

        let parts: Vec<&str> = parts[0].split(':').map(|p| p.trim()).collect();
        if parts.len() == 1 {
            match parts[0] {
                "rtlsdr" => {
                    source.rtlsdr = true;
                }
                s if {
                    if let Ok(s) = s.parse::<u16>() {
                        source.host = "0.0.0.0".to_string();
                        source.port = s;
                        true
                    } else {
                        false
                    }
                } => {}
                _ => {
                    return Err("unparsable".to_string());
                }
            }
        } else {
            source.host = parts[0].to_string();
            if let Ok(port) = parts[1].parse::<u16>() {
                source.port = port;
            } else {
                return Err("unparsable port".to_string());
            }
        }
        Ok(source)
    }
}

impl Source {
    pub async fn receiver(&self, tx: Sender<TimedMessage>, idx: usize) {
        if self.rtlsdr {
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
            let server_address = format!("{}:{}", self.host, self.port);
            radarcape::receiver(server_address, tx, idx).await
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_source() {
        let source = Source::from_str("rtlsdr");
        assert!(source.is_ok());
        if let Ok(Source { rtlsdr, .. }) = source {
            assert!(rtlsdr);
        }

        let source = Source::from_str("rtlsdr@LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            rtlsdr,
            airport,
            reference: Some(pos),
            ..
        }) = source
        {
            assert!(rtlsdr);
            assert_eq!(airport, Some("LFBO".to_string()));
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }
        let source = Source::from_str("foo@LFBO");
        assert!(source.is_err());

        let source = Source::from_str("4003");
        assert!(source.is_ok());
        if let Ok(Source {
            host,
            port,
            rtlsdr,
            airport,
            reference,
            ..
        }) = source
        {
            assert!(!rtlsdr);
            assert_eq!(host, "0.0.0.0");
            assert_eq!(port, 4003);
            assert_eq!(airport, Some("LFBO".to_string()));
            assert_eq!(reference, None);
        }

        let source = Source::from_str("4003@LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            host,
            port,
            rtlsdr,
            airport,
            reference: Some(pos),
            ..
        }) = source
        {
            assert!(!rtlsdr);
            assert_eq!(host, "0.0.0.0");
            assert_eq!(port, 4003);
            assert_eq!(airport, Some("LFBO".to_string()));
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }

        let source = Source::from_str("1.2.3.4:4003@LFBO");
        assert!(source.is_ok());
        if let Ok(Source {
            host,
            port,
            rtlsdr,
            airport,
            reference: Some(pos),
            ..
        }) = source
        {
            assert!(!rtlsdr);
            assert_eq!(host, "1.2.3.4");
            assert_eq!(port, 4003);
            assert_eq!(airport, Some("LFBO".to_string()));
            assert_eq!(pos.latitude, 43.628101);
            assert_eq!(pos.longitude, 1.367263);
        }
    }
}
