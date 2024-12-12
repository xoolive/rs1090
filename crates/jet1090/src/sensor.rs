use rs1090::prelude::Position;
#[cfg(feature = "sero")]
use rs1090::source::sero;
use serde::{Deserialize, Serialize};

use crate::source::{Address, Source};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensor {
    pub serial: u64,
    pub name: Option<String>,
    pub reference: Option<Position>,
    pub count: u64, // should be aircraft_count
    pub last: u64,  // should be last timestamp
}

pub async fn sensors(value: &Source) -> Vec<Sensor> {
    match &value.address {
        Address::Tcp(_)
        | Address::Udp(_)
        | Address::Websocket(_)
        | Address::Rtlsdr(_) => {
            vec![Sensor {
                serial: value.serial(),
                name: value.name.clone(),
                reference: value.reference,
                count: 0,
                last: 0,
            }]
        }
        Address::Sero(params) => {
            #[cfg(feature = "sero")]
            {
                let sero = sero::SeroClient::from(params);
                let info = sero.info().await.unwrap();
                info.sensor_info
                    .iter()
                    .map(|elt| Sensor {
                        serial: elt.sensor.unwrap().serial,
                        reference: elt.gnss.as_ref().unwrap().position.map(
                            |pos| Position {
                                latitude: pos.latitude,
                                longitude: pos.longitude,
                            },
                        ),
                        name: Some(elt.alias.to_string()),
                        count: 0,
                        last: 0,
                    })
                    .collect()
            }
            #[cfg(not(feature = "sero"))]
            {
                vec![]
            }
        }
    }
}
