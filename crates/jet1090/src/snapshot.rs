use rs1090::decode::bds::bds09::AirborneVelocitySubType::{
    AirspeedSubsonic, GroundSpeedDecoding,
};
use rs1090::decode::bds::bds09::AirspeedType::{IAS, TAS};
use rs1090::decode::IdentityCode;
use rs1090::prelude::*;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::Jet1090;

#[derive(Debug)]
pub struct StateVectors {
    pub cur: Snapshot,
    pub hist: Vec<TimedMessage>,
}

impl StateVectors {
    fn new(ts: u64, icao24: String) -> StateVectors {
        let hexid = u32::from_str_radix(&icao24, 16).unwrap_or(0);
        let cur = Snapshot {
            icao24,
            first: ts,
            last: ts,
            callsign: None,
            registration: rs1090::data::tail::tail(hexid),
            typecode: None,
            squawk: None,
            latitude: None,
            longitude: None,
            altitude: None,
            selected_altitude: None,
            groundspeed: None,
            vertical_rate: None,
            track: None,
            ias: None,
            tas: None,
            mach: None,
            roll: None,
            heading: None,
            nacp: None,
        };
        StateVectors {
            cur,
            hist: Vec::<TimedMessage>::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub icao24: String,
    pub first: u64,
    pub last: u64,
    pub callsign: Option<String>,
    pub registration: Option<String>,
    pub typecode: Option<String>,
    pub squawk: Option<IdentityCode>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<u16>,
    pub selected_altitude: Option<u16>,
    pub groundspeed: Option<f64>,
    pub vertical_rate: Option<i16>,
    pub track: Option<f64>,
    pub ias: Option<u16>,
    pub tas: Option<u16>,
    pub mach: Option<f64>,
    pub roll: Option<f64>,
    pub heading: Option<f64>,
    pub nacp: Option<u8>,
}

fn icao24(msg: &Message) -> Option<String> {
    match &msg.df {
        ShortAirAirSurveillance { ap, .. } => Some(ap.to_string()),
        SurveillanceAltitudeReply { ap, .. } => Some(ap.to_string()),
        SurveillanceIdentityReply { ap, .. } => Some(ap.to_string()),
        AllCallReply { icao, .. } => Some(icao.to_string()),
        LongAirAirSurveillance { ap, .. } => Some(ap.to_string()),
        ExtendedSquitterADSB(ADSB { icao24, .. }) => Some(icao24.to_string()),
        ExtendedSquitterTisB { cf, .. } => Some(cf.aa.to_string()),
        CommBAltitudeReply { ap, .. } => Some(ap.to_string()),
        CommBIdentityReply { ap, .. } => Some(ap.to_string()),
        _ => None,
    }
}

pub async fn update_snapshot(states: &Mutex<Jet1090>, msg: &mut TimedMessage) {
    if let TimedMessage {
        timestamp,
        message: Some(message),
        ..
    } = msg
    {
        if let Some(icao24) = icao24(message) {
            let states = &mut states.lock().await.state_vectors;
            let aircraft = states
                .entry(icao24.to_string())
                .or_insert(StateVectors::new(*timestamp as u64, icao24));
            aircraft.cur.last = *timestamp as u64;

            match &mut message.df {
                SurveillanceIdentityReply { id, .. } => {
                    aircraft.cur.squawk = Some(*id)
                }
                SurveillanceAltitudeReply { ac, .. } => {
                    aircraft.cur.altitude = Some(ac.0);
                }
                ExtendedSquitterADSB(adsb) => match &adsb.message {
                    ME::BDS05(bds05) => {
                        aircraft.cur.latitude = bds05.latitude;
                        aircraft.cur.longitude = bds05.longitude;
                        aircraft.cur.altitude = bds05.alt;
                    }
                    ME::BDS06(bds06) => {
                        aircraft.cur.latitude = bds06.latitude;
                        aircraft.cur.longitude = bds06.longitude;
                        aircraft.cur.track = bds06.track;
                        aircraft.cur.groundspeed = bds06.groundspeed;
                        aircraft.cur.altitude = None;
                    }
                    ME::BDS08(bds08) => {
                        aircraft.cur.callsign = Some(bds08.callsign.to_string())
                    }
                    ME::BDS09(bds09) => {
                        aircraft.cur.vertical_rate = bds09.vertical_rate;
                        match &bds09.velocity {
                            GroundSpeedDecoding(spd) => {
                                aircraft.cur.groundspeed =
                                    Some(spd.groundspeed);
                                aircraft.cur.track = Some(spd.track)
                            }
                            AirspeedSubsonic(spd) => {
                                match spd.airspeed_type {
                                    IAS => aircraft.cur.ias = spd.airspeed,
                                    TAS => aircraft.cur.tas = spd.airspeed,
                                }
                                aircraft.cur.heading = spd.heading;
                            }
                            _ => {}
                        }
                    }
                    ME::BDS61(bds61) => {
                        aircraft.cur.squawk = Some(bds61.squawk);
                    }
                    ME::BDS62(bds62) => {
                        aircraft.cur.selected_altitude =
                            bds62.selected_altitude;
                        aircraft.cur.nacp = Some(bds62.nac_p);
                    }
                    ME::BDS65(bds65) => match bds65 {
                        AircraftOperationStatus::Airborne(st) => {
                            match st.version {
                                rs1090::decode::bds::bds65::ADSBVersionAirborne::DOC9871AppendixB(v) => {
                                    aircraft.cur.nacp = Some(v.nac_p)
                                }
                                rs1090::decode::bds::bds65::ADSBVersionAirborne::DOC9871AppendixC(v) => {
                                    aircraft.cur.nacp = Some(v.nac_p)
                                }
                                _ => {}
                            }
                        }
                        AircraftOperationStatus::Surface(st) => {
                            match st.version {
                                rs1090::decode::bds::bds65::ADSBVersionSurface::DOC9871AppendixB(v) => {
                                    aircraft.cur.nacp = Some(v.nac_p)
                                }
                                rs1090::decode::bds::bds65::ADSBVersionSurface::DOC9871AppendixC(v) => {
                                    aircraft.cur.nacp = Some(v.nac_p)
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                },
                ExtendedSquitterTisB { cf, .. } => {
                    aircraft.cur.typecode = Some("GRND".to_string());
                    match &cf.me {
                        ME::BDS05(bds05) => {
                            aircraft.cur.latitude = bds05.latitude;
                            aircraft.cur.longitude = bds05.longitude;
                            aircraft.cur.altitude = bds05.alt;
                        }
                        ME::BDS06(bds06) => {
                            aircraft.cur.latitude = bds06.latitude;
                            aircraft.cur.longitude = bds06.longitude;
                            aircraft.cur.track = bds06.track;
                            aircraft.cur.groundspeed = bds06.groundspeed;
                            aircraft.cur.altitude = None;
                        }
                        ME::BDS08(bds08) => {
                            aircraft.cur.callsign =
                                Some(bds08.callsign.to_string())
                        }
                        _ => {}
                    }
                }
                CommBAltitudeReply { bds, .. } => {
                    // Invalidate data if marked as both BDS50 and BDS60
                    if let (Some(_), Some(_)) = (&bds.bds50, &bds.bds60) {
                        bds.bds50 = None;
                        bds.bds60 = None
                    }
                    if let Some(bds20) = &bds.bds20 {
                        aircraft.cur.callsign =
                            Some(bds20.callsign.to_string());
                    }
                    if let Some(bds40) = &bds.bds40 {
                        aircraft.cur.selected_altitude =
                            bds40.selected_altitude_mcp;
                    }
                    if let Some(bds50) = &bds.bds50 {
                        aircraft.cur.roll = bds50.roll_angle;
                        aircraft.cur.track = bds50.track_angle;
                        aircraft.cur.groundspeed =
                            bds50.groundspeed.map(|x| x as f64);
                        aircraft.cur.tas = bds50.true_airspeed;
                    }
                    if let Some(bds60) = &bds.bds60 {
                        aircraft.cur.ias = bds60.indicated_airspeed;
                        aircraft.cur.mach = bds60.mach_number;
                        aircraft.cur.heading = bds60.magnetic_heading;
                        if bds60.inertial_vertical_velocity.is_some() {
                            aircraft.cur.vertical_rate =
                                bds60.inertial_vertical_velocity;
                        }
                    }
                }
                CommBIdentityReply { bds, .. } => {
                    // Invalidate data if marked as both BDS50 and BDS60
                    if let (Some(_), Some(_)) = (&bds.bds50, &bds.bds60) {
                        bds.bds50 = None;
                        bds.bds60 = None
                    }
                    if let Some(bds20) = &bds.bds20 {
                        aircraft.cur.callsign =
                            Some(bds20.callsign.to_string());
                    }
                    if let Some(bds40) = &bds.bds40 {
                        aircraft.cur.selected_altitude =
                            bds40.selected_altitude_mcp;
                    }
                    if let Some(bds50) = &bds.bds50 {
                        aircraft.cur.roll = bds50.roll_angle;
                        aircraft.cur.track = bds50.track_angle;
                        aircraft.cur.groundspeed =
                            bds50.groundspeed.map(|x| x as f64);
                        aircraft.cur.tas = bds50.true_airspeed;
                    }
                    if let Some(bds60) = &bds.bds60 {
                        aircraft.cur.ias = bds60.indicated_airspeed;
                        aircraft.cur.mach = bds60.mach_number;
                        aircraft.cur.heading = bds60.magnetic_heading;
                        if bds60.inertial_vertical_velocity.is_some() {
                            aircraft.cur.vertical_rate =
                                bds60.inertial_vertical_velocity;
                        }
                    }
                }
                _ => {}
            };
        }
    }
}

pub async fn store_history(states: &Mutex<Jet1090>, msg: TimedMessage) {
    if let TimedMessage {
        timestamp,
        message: Some(message),
        idx,
        ..
    } = msg
    {
        if let Some(icao24) = icao24(&message) {
            let states = &mut states.lock().await.state_vectors;
            let aircraft = states
                .entry(icao24.to_string())
                .or_insert(StateVectors::new(timestamp as u64, icao24));

            match message.df {
                ExtendedSquitterADSB(_)
                | ExtendedSquitterTisB { .. }
                | CommBAltitudeReply { .. }
                | CommBIdentityReply { .. } => {
                    aircraft.hist.push(TimedMessage {
                        timestamp,
                        frame: "".to_string(),
                        message: Some(message),
                        idx,
                    })
                }
                _ => {}
            }
        }
    }
}
