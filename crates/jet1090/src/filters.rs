use rs1090::decode::{TimedMessage, ICAO};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Filters {
    pub df_filter: Option<Vec<String>>,
    pub aircraft_filter: Option<Vec<ICAO>>,
    //pub sensor_filter: Option<Vec<String>>,
}

impl Filters {
    fn aircraft_in<T>(filter: &Filters, icao24: &T) -> bool
    where
        T: Copy + Into<ICAO>,
    {
        if let Some(filter) = &filter.aircraft_filter {
            if filter.contains(&(*icao24).into()) {
                return true;
            }
            return filter.is_empty();
        }
        true
    }

    fn df_in(filter: &Filters, df: &str) -> bool {
        if let Some(filter) = &filter.df_filter {
            if filter.contains(&df.to_string()) {
                return true;
            }
            return filter.is_empty();
        }
        true
    }

    pub fn is_in(filter: &Filters, msg: &TimedMessage) -> bool {
        if let Some(msg) = &msg.message {
            match &msg.df {
                rs1090::decode::DF::ShortAirAirSurveillance { ap, .. } => {
                    if Self::aircraft_in(filter, ap) {
                        return Self::df_in(filter, "0");
                    }
                }
                rs1090::decode::DF::SurveillanceAltitudeReply {
                    ap, ..
                } => {
                    if Self::aircraft_in(filter, ap) {
                        return Self::df_in(filter, "4");
                    }
                }
                rs1090::decode::DF::SurveillanceIdentityReply {
                    ap, ..
                } => {
                    if Self::aircraft_in(filter, ap) {
                        return Self::df_in(filter, "5");
                    }
                }
                rs1090::decode::DF::AllCallReply { icao, .. } => {
                    if Self::aircraft_in(filter, icao) {
                        return Self::df_in(filter, "11");
                    }
                }
                rs1090::decode::DF::LongAirAirSurveillance { ap, .. } => {
                    if Self::aircraft_in(filter, ap) {
                        return Self::df_in(filter, "16");
                    }
                }
                rs1090::decode::DF::ExtendedSquitterADSB(adsb) => {
                    if Self::aircraft_in(filter, &adsb.icao24) {
                        return Self::df_in(filter, "17");
                    }
                }
                rs1090::decode::DF::ExtendedSquitterTisB { pi, .. } => {
                    if Self::aircraft_in(filter, pi) {
                        return Self::df_in(filter, "18");
                    }
                }
                rs1090::decode::DF::ExtendedSquitterMilitary { .. } => {
                    return Self::df_in(filter, "19");
                }
                rs1090::decode::DF::CommBAltitudeReply { ap, .. } => {
                    if Self::aircraft_in(filter, ap) {
                        return Self::df_in(filter, "20");
                    }
                }
                rs1090::decode::DF::CommBIdentityReply { ap, .. } => {
                    if Self::aircraft_in(filter, ap) {
                        return Self::df_in(filter, "21");
                    }
                }
                rs1090::decode::DF::CommDExtended { parity, .. } => {
                    if Self::aircraft_in(filter, parity) {
                        return Self::df_in(filter, "24");
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rs1090::decode::Message;

    #[test]
    fn test_filter() {
        let mut tmsg = TimedMessage {
            timestamp: 0.,
            frame: hex::decode("8c4841753a9a153237aef0f275be").unwrap(),
            message: None,
            metadata: vec![],
            decode_time: None,
        };
        tmsg.message = Message::try_from(tmsg.frame.as_slice()).ok();

        let toml_data = r#"
        df_filter = []
        aircraft_filter = []
        "#;
        let filter: Filters =
            toml::from_str(toml_data).expect("Failed to deserialize TOML");

        assert!(Filters::is_in(&filter, &tmsg));

        let toml_data = r#"
            df_filter = ["17", "20", "21"]
            aircraft_filter = []
        "#;
        let filter: Filters =
            toml::from_str(toml_data).expect("Failed to deserialize TOML");

        assert!(Filters::is_in(&filter, &tmsg));

        let toml_data = r#"
            df_filter = ["17", "20", "21"]
            aircraft_filter = ["484175"]
        "#;
        let filter: Filters =
            toml::from_str(toml_data).expect("Failed to deserialize TOML");

        assert!(Filters::is_in(&filter, &tmsg));

        let toml_data = r#"
            df_filter = ["11"]
            aircraft_filter = ["484175"]
        "#;
        let filter: Filters =
            toml::from_str(toml_data).expect("Failed to deserialize TOML");

        assert!(!Filters::is_in(&filter, &tmsg));

        let toml_data = r#"
            df_filter = ["17", "20", "21"]
            aircraft_filter = ["333333"]
        "#;
        let filter: Filters =
            toml::from_str(toml_data).expect("Failed to deserialize TOML");

        assert!(!Filters::is_in(&filter, &tmsg));

        let mut tmsg = TimedMessage {
            timestamp: 1735943148.353877,
            frame: hex::decode("02c18c3b323e4f").unwrap(),
            message: None,
            metadata: vec![],
            decode_time: None,
        };
        tmsg.message = Message::try_from(tmsg.frame.as_slice()).ok();

        let toml_data = r#"
            df_filter = ["17", "20", "21"]
        "#;
        let filter: Filters =
            toml::from_str(toml_data).expect("Failed to deserialize TOML");

        assert!(!Filters::is_in(&filter, &tmsg));

        let toml_data = r#"
            df_filter = ["0"]
        "#;
        let filter: Filters =
            toml::from_str(toml_data).expect("Failed to deserialize TOML");

        assert!(Filters::is_in(&filter, &tmsg));
    }
}
