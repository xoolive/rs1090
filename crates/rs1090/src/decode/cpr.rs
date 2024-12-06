/**
* The position information is encoded in a Compact Position Reporting (CPR)
* format, which requires fewer bits to encode positions with higher resolution.
* The CPR offers a trade-off between global position ambiguity and local
* position accuracy. Two types of position messages (identified by the odd and
* even frame bit) are broadcast alternately.
*
* There are two different ways to decode an airborne position:
*
*  - globally unambiguous position decoding: Without a known position to start
*    with, using both types of messages to decode the position.
*  - locally unambiguous position decoding: Knowing a reference position from
*    previous sets of messages, using only one message for the decoding.
*
*/
use super::adsb::ME;
use super::bds::bds05::AirbornePosition;
use super::bds::bds06::SurfacePosition;
use super::{TimedMessage, DF, ICAO};
use crate::data::airports::one_airport;
use deku::prelude::*;
use libm::fabs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin() * (d_lat / 2.0).sin()
        + lat1.to_radians().cos()
            * lat2.to_radians().cos()
            * (d_lon / 2.0).sin()
            * (d_lon / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    const R: f64 = 6371.0; // Earth's radius in kilometers
    R * c // Distance in kilometers
}

fn dist_haversine(pos1: &Position, pos2: &Position) -> f64 {
    haversine(pos1.latitude, pos1.longitude, pos2.latitude, pos2.longitude)
}

/// A flag to qualify a CPR position as odd or even
#[derive(Debug, PartialEq, Eq, Serialize, DekuRead, Copy, Clone)]
#[deku(id_type = "u8", bits = "1")]
#[serde(rename_all = "snake_case")]
pub enum CPRFormat {
    Even = 0,
    Odd = 1,
}

impl fmt::Display for CPRFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Even => "even",
                Self::Odd => "odd",
            }
        )
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
}

impl FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let regex_list = [Regex::new(s).unwrap()];
        if let Some(airport) = one_airport(&regex_list) {
            return Ok(Position {
                latitude: airport.lat,
                longitude: airport.lon,
            });
        }
        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();

        if parts.len() != 2 {
            return Err("Invalid number of coordinates".to_string());
        }

        let latitude: f64 = parts[0]
            .parse()
            .map_err(|e| format!("Latitude parse error: {}", e))?;
        let longitude: f64 = parts[1]
            .parse()
            .map_err(|e| format!("Longitude parse error: {}", e))?;

        Ok(Position {
            latitude,
            longitude,
        })
    }
}

#[derive(Default)]
pub struct AircraftState {
    timestamp: f64,
    pos: Option<Position>,
    odd_ts: f64,
    odd_msg: Option<AirbornePosition>,
    even_ts: f64,
    even_msg: Option<AirbornePosition>,
}

/// NZ represents the number of latitude zones between the equator and a pole.
/// In Mode S, is defined to be 15.
const NZ: f64 = 15.0;

/// CPR_MAX is 2^17 since CPR lat and lon values are encoded on 17 bits
const CPR_MAX: f64 = 131_072.0;

/// Given the latitude, this function yields the number of longitude zones
/// between 1 and 59.
/// The nl function uses the precomputed table from 1090-WP-9-14
#[rustfmt::skip]
fn nl(lat: f64) -> u64 {
    let mut lat = lat;
    if lat < 0.0 { lat = -lat; }
    if lat < 29.911_356_86 {
        if lat < 10.470_471_30 { return 59; }
        if lat < 14.828_174_37 { return 58; }
        if lat < 18.186_263_57 { return 57; }
        if lat < 21.029_394_93 { return 56; }
        if lat < 23.545_044_87 { return 55; }
        if lat < 25.829_247_07 { return 54; }
        if lat < 27.938_987_10 { return 53; }
        // < 29.91135686
        return 52;
    }
    if lat < 44.194_549_51 {
        if lat < 31.772_097_08 { return 51; }
        if lat < 33.539_934_36 { return 50; }
        if lat < 35.228_995_98 { return 49; }
        if lat < 36.850_251_08 { return 48; }
        if lat < 38.412_418_92 { return 47; }
        if lat < 39.922_566_84 { return 46; }
        if lat < 41.386_518_32 { return 45; }
        if lat < 42.809_140_12 { return 44; }
        // < 44.19454951
        return 43;
    }
    if lat < 59.954_592_77 {
        if lat < 45.546_267_23 { return 42; }
        if lat < 46.867_332_52 { return 41; }
        if lat < 48.160_391_28 { return 40; }
        if lat < 49.427_764_39 { return 39; }
        if lat < 50.671_501_66 { return 38; }
        if lat < 51.893_424_69 { return 37; }
        if lat < 53.095_161_53 { return 36; }
        if lat < 54.278_174_72 { return 35; }
        if lat < 55.443_784_44 { return 34; }
        if lat < 56.593_187_56 { return 33; }
        if lat < 57.727_473_54 { return 32; }
        if lat < 58.847_637_76 { return 31; }
        // < 59.95459277
        return 30;
    }
    if lat < 61.049_177_74 { return 29; }
    if lat < 62.132_166_59 { return 28; }
    if lat < 63.204_274_79 { return 27; }
    if lat < 64.266_165_23 { return 26; }
    if lat < 65.318_453_10 { return 25; }
    if lat < 66.361_710_08 { return 24; }
    if lat < 67.396_467_74 { return 23; }
    if lat < 68.423_220_22 { return 22; }
    if lat < 69.442_426_31 { return 21; }
    if lat < 70.454_510_75 { return 20; }
    if lat < 71.459_864_73 { return 19; }
    if lat < 72.458_845_45 { return 18; }
    if lat < 73.451_774_42 { return 17; }
    if lat < 74.438_934_16 { return 16; }
    if lat < 75.420_562_57 { return 15; }
    if lat < 76.396_843_91 { return 14; }
    if lat < 77.367_894_61 { return 13; }
    if lat < 78.333_740_83 { return 12; }
    if lat < 79.294_282_25 { return 11; }
    if lat < 80.249_232_13 { return 10; }
    if lat < 81.198_013_49 { return 9; }
    if lat < 82.139_569_81 { return 8; }
    if lat < 83.071_994_45 { return 7; }
    if lat < 83.991_735_63 { return 6; }
    if lat < 84.891_661_91 { return 5; }
    if lat < 85.755_416_21 { return 4; }
    if lat < 86.535_369_98 { return 3; }
    if lat < 87.000_000_00 { return 2; }
    1
}

const D_LAT_EVEN: f64 = 360.0 / (4.0 * NZ);
const D_LAT_ODD: f64 = 360.0 / (4.0 * NZ - 1.0);

// Main difference for % between Python and Rust is that in Rust, the sign
// of the result matches the sign of the dividend.
fn modulo(a: f64, b: f64) -> f64 {
    if a >= 0. {
        a % b
    } else {
        a % b + libm::fabs(b)
    }
}

/**
 * Decode airborne position from a pair of even and odd position message.
 */
pub fn airborne_position(
    oldest: &AirbornePosition,
    latest: &AirbornePosition,
) -> Option<Position> {
    let (even_frame, odd_frame) = match (oldest, latest) {
        (
            even @ AirbornePosition {
                parity: CPRFormat::Even,
                ..
            },
            odd @ AirbornePosition {
                parity: CPRFormat::Odd,
                ..
            },
        )
        | (
            odd @ AirbornePosition {
                parity: CPRFormat::Odd,
                ..
            },
            even @ AirbornePosition {
                parity: CPRFormat::Even,
                ..
            },
        ) => (even, odd),
        _ => return None,
    };

    let cpr_lat_even = f64::from(even_frame.lat_cpr) / CPR_MAX;
    let cpr_lon_even = f64::from(even_frame.lon_cpr) / CPR_MAX;
    let cpr_lat_odd = f64::from(odd_frame.lat_cpr) / CPR_MAX;
    let cpr_lon_odd = f64::from(odd_frame.lon_cpr) / CPR_MAX;

    let j = libm::floor(59.0 * cpr_lat_even - 60.0 * cpr_lat_odd + 0.5);

    let mut lat_even = D_LAT_EVEN * (modulo(j, 60.) + cpr_lat_even);
    let mut lat_odd = D_LAT_ODD * (modulo(j, 59.) + cpr_lat_odd);

    if lat_even >= 270.0 {
        lat_even -= 360.0;
    }

    if lat_odd >= 270.0 {
        lat_odd -= 360.0;
    }

    if !(-90. ..=90.).contains(&lat_even) || !(-90. ..=90.).contains(&lat_odd) {
        return None;
    }
    if nl(lat_even) != nl(lat_odd) {
        return None;
    }

    let lat = if latest == even_frame {
        lat_even
    } else {
        lat_odd
    };
    let cpr_format = &latest.parity;

    let (p, c) = if cpr_format == &CPRFormat::Even {
        (0, cpr_lon_even)
    } else {
        (1, cpr_lon_odd)
    };
    let ni = std::cmp::max(nl(lat) - p, 1) as f64;
    let m = libm::floor(
        cpr_lon_even * (nl(lat) - 1) as f64 - cpr_lon_odd * nl(lat) as f64
            + 0.5,
    );

    let r = modulo(m, ni);

    let mut lon = (360.0 / ni) * (r + c);
    if lon >= 180.0 {
        lon -= 360.0;
    }

    Some(Position {
        latitude: lat,
        longitude: lon,
    })
}

/**
 * Decode airborne position with only one message, knowing reference nearby
 * location, such as previously calculated location, ground station, or airport
 * location, etc. The reference position shall be within 180NM of the true
 * position.
 */
pub fn airborne_position_with_reference(
    msg: &AirbornePosition,
    latitude_ref: f64,
    longitude_ref: f64,
) -> Option<Position> {
    let cpr_lat = f64::from(msg.lat_cpr) / CPR_MAX;
    let cpr_lon = f64::from(msg.lon_cpr) / CPR_MAX;

    let d_lat = if msg.parity == CPRFormat::Even {
        360. / 60.
    } else {
        360. / 59.
    };

    let j = libm::floor(latitude_ref / d_lat)
        + libm::floor(0.5 + modulo(latitude_ref, d_lat) / d_lat - cpr_lat);

    let lat = d_lat * (j + cpr_lat);

    if !(-90. ..=90.).contains(&lat) {
        return None;
    }
    // Check that the answer is not more than half a cell away
    if fabs(lat - latitude_ref) > d_lat / 2. {
        return None;
    }

    let ni = if msg.parity == CPRFormat::Even {
        nl(lat)
    } else {
        nl(lat) - 1
    };
    let d_lon = if ni > 0 { 360. / ni as f64 } else { 360. };
    let m = libm::floor(longitude_ref / d_lon)
        + libm::floor(0.5 + modulo(longitude_ref, d_lon) / d_lon - cpr_lon);
    let lon = d_lon * (m + cpr_lon);

    // Check that the answer is not more than half a cell away
    if fabs(lon - longitude_ref) > d_lon / 2. {
        return None;
    }

    Some(Position {
        latitude: lat,
        longitude: lon,
    })
}

/**
 * Decode surface position with only one message, knowing reference nearby
 * location, such as previously calculated location, ground station, or airport
 * location, etc. The reference position shall be within 45NM of the true
 * position.
 */
pub fn surface_position_with_reference(
    msg: &SurfacePosition,
    latitude_ref: f64,
    longitude_ref: f64,
) -> Option<Position> {
    let cpr_lat = f64::from(msg.lat_cpr) / CPR_MAX;
    let cpr_lon = f64::from(msg.lon_cpr) / CPR_MAX;

    let d_lat = if msg.parity == CPRFormat::Even {
        90. / 60.
    } else {
        90. / 59.
    };

    let j = libm::floor(latitude_ref / d_lat)
        + libm::floor(0.5 + modulo(latitude_ref, d_lat) / d_lat - cpr_lat);

    let lat = d_lat * (j + cpr_lat);

    if !(-90. ..=90.).contains(&lat) {
        return None;
    }
    // Check that the answer is not more than half a cell away
    if fabs(lat - latitude_ref) > d_lat / 2. {
        return None;
    }

    let ni = if msg.parity == CPRFormat::Even {
        nl(lat)
    } else {
        nl(lat) - 1
    };
    let d_lon = if ni > 0 { 90. / ni as f64 } else { 90. };
    let m = libm::floor(longitude_ref / d_lon)
        + libm::floor(0.5 + modulo(longitude_ref, d_lon) / d_lon - cpr_lon);
    let lon = d_lon * (m + cpr_lon);

    // Check that the answer is not more than half a cell away
    if fabs(lon - longitude_ref) > d_lon / 2. {
        return None;
    }

    Some(Position {
        latitude: lat,
        longitude: lon,
    })
}

pub type UpdateIf = Option<Box<dyn Fn(&AirbornePosition) -> bool>>;

/**
 * Mutates the ME message based on recent past positions (parameter `timestamp`)
 * of the same aircraft (parameter `icao24`). For surface messages, the
 * reference position will be considered; and possibly updated based on low
 * altitude positions detected.
 *
 * - `aircraft` is a hashmap of aircraft containing their most recent state;
 * - `reference` is a (possibly None) set of coordinates.
 */
pub fn decode_position(
    message: &mut ME,
    timestamp: f64,
    icao24: &ICAO,
    aircraft: &mut BTreeMap<ICAO, AircraftState>,
    reference: &mut Option<Position>,
    update_reference: &UpdateIf,
) {
    let latest = aircraft.entry(*icao24).or_insert(AircraftState {
        timestamp,
        pos: None,
        odd_ts: timestamp,
        odd_msg: None,
        even_ts: timestamp,
        even_msg: None,
    });
    match message {
        ME::BDS05(airborne) => {
            let mut pos: Option<Position> = None;

            let latest_timestamp = match airborne.parity {
                CPRFormat::Even => latest.odd_ts,
                CPRFormat::Odd => latest.even_ts,
            };
            let latest_msg = match airborne.parity {
                CPRFormat::Even => latest.odd_msg,
                CPRFormat::Odd => latest.even_msg,
            };

            // This may happen with several sources of data coming on one mpsc
            if (timestamp - latest_timestamp) < 0. {
                return;
            }

            if (timestamp - latest_timestamp) < 10. {
                // First decoding based on odd/even (global)
                // This is the most reasonable way to decode
                pos = match latest_msg {
                    Some(oldest) => airborne_position(&oldest, airborne),
                    None => None,
                };
            }

            // If failed try to use previous reference
            // This is tricky though, use with extra care
            if pos.is_none() & ((timestamp - latest.timestamp) < 180.) {
                if let Some(latest_pos) = latest.pos {
                    pos = airborne_position_with_reference(
                        airborne,
                        latest_pos.latitude,
                        latest_pos.longitude,
                    )
                }
            }

            if let Some(new_pos) = pos {
                if let Some(latest_pos) = latest.pos {
                    // Invalidate if new position is not reasonable
                    if dist_haversine(&new_pos, &latest_pos) > 50. {
                        pos = None
                    }
                }
            }

            if let Some(pos) = pos {
                // First update the message
                airborne.latitude = Some(pos.latitude);
                airborne.longitude = Some(pos.longitude);
                // Then update the reference in aircraft
                latest.pos = Some(pos);
                latest.timestamp = timestamp;
                // If necessary (according to the callback) update the reference position
                if let Some(update_reference) = update_reference {
                    if update_reference(airborne) {
                        *reference = Some(Position {
                            latitude: pos.latitude,
                            longitude: pos.longitude,
                        })
                    }
                }
            } else {
                latest.pos = None;
            }

            match airborne.parity {
                CPRFormat::Even => {
                    latest.even_msg = Some(*airborne);
                    latest.even_ts = timestamp
                }
                CPRFormat::Odd => {
                    latest.odd_msg = Some(*airborne);
                    latest.odd_ts = timestamp
                }
            }
        }
        ME::BDS06(surface) => {
            let mut pos = None;
            if let Some(latest_pos) = latest.pos {
                let surface_pos = surface_position_with_reference(
                    surface,
                    latest_pos.latitude,
                    latest_pos.longitude,
                );
                if surface_pos.is_some()
                    && dist_haversine(&latest_pos, &surface_pos.unwrap()) < 1.
                {
                    pos = surface_pos;
                }
            }
            if let Some(reference) = reference {
                if pos.is_none() {
                    pos = surface_position_with_reference(
                        surface,
                        reference.latitude,
                        reference.longitude,
                    )
                }
            }
            if let Some(pos) = pos {
                // First update the message
                surface.latitude = Some(pos.latitude);
                surface.longitude = Some(pos.longitude);
                // Then update the reference in aircraft
                latest.pos = Some(pos);
                latest.timestamp = timestamp;
            }
        }
        _ => (),
    }
}

/**
 * This function is only used  for the decoding of offline messages.
 */
pub fn decode_positions(
    res: &mut [TimedMessage],
    reference: Option<Position>,
    update_reference: &UpdateIf,
) {
    let mut aircraft: BTreeMap<ICAO, AircraftState> = BTreeMap::new();
    let mut reference = reference;

    let _: Vec<()> = res
        .iter_mut()
        .map(|msg| {
            if let Some(message) = &mut msg.message {
                match &mut message.df {
                    DF::ExtendedSquitterADSB(adsb) => decode_position(
                        &mut adsb.message,
                        msg.timestamp,
                        &adsb.icao24,
                        &mut aircraft,
                        &mut reference,
                        update_reference,
                    ),
                    DF::ExtendedSquitterTisB { cf, .. } => decode_position(
                        &mut cf.me,
                        msg.timestamp,
                        &cf.aa,
                        &mut aircraft,
                        &mut reference,
                        update_reference,
                    ),
                    _ => {}
                }
            }
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn decode_airporne_position() {
        let b1 = hex!("8D40058B58C901375147EFD09357");
        let b2 = hex!("8D40058B58C904A87F402D3B8C59");
        let (_, msg1) = Message::from_bytes((&b1, 0)).unwrap();
        let (_, msg2) = Message::from_bytes((&b2, 0)).unwrap();

        let (msg1, msg2) = match (msg1.df, msg2.df) {
            (ExtendedSquitterADSB(msg1), ExtendedSquitterADSB(msg2)) => {
                match (msg1.message, msg2.message) {
                    (ME::BDS05(m1), ME::BDS05(m2)) => (m1, m2),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };

        let Position {
            latitude,
            longitude,
        } = airborne_position(&msg1, &msg2).unwrap();

        assert_relative_eq!(latitude, 49.81755, max_relative = 1e-3);
        assert_relative_eq!(longitude, 6.08442, max_relative = 1e-3);

        let b3 = hex!("8d4d224f58bf07c2d41a9a353d70");
        let b4 = hex!("8d4d224f58bf003b221b34aa5b8d");

        let (_, msg1) = Message::from_bytes((&b3, 0)).unwrap();
        let (_, msg2) = Message::from_bytes((&b4, 0)).unwrap();

        let (msg1, msg2) = match (msg1.df, msg2.df) {
            (ExtendedSquitterADSB(msg1), ExtendedSquitterADSB(msg2)) => {
                match (msg1.message, msg2.message) {
                    (ME::BDS05(m1), ME::BDS05(m2)) => (m1, m2),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };

        let Position {
            latitude,
            longitude,
        } = airborne_position(&msg1, &msg2).unwrap();

        assert_relative_eq!(latitude, 42.346, max_relative = 1e-3);
        assert_relative_eq!(longitude, 0.4347, max_relative = 1e-3);
    }

    #[test]
    fn decode_airporne_position_with_reference() {
        let bytes = hex!("8D40058B58C901375147EFD09357");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();

        let msg = match msg.df {
            ExtendedSquitterADSB(msg) => match msg.message {
                ME::BDS05(me) => me,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        let Position {
            latitude,
            longitude,
        } = airborne_position_with_reference(&msg, 49.0, 6.0).unwrap();

        assert_relative_eq!(latitude, 49.82410, max_relative = 1e-3);
        assert_relative_eq!(longitude, 6.06785, max_relative = 1e-3);

        let bytes = hex!("8D40058B58C904A87F402D3B8C59");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();

        let msg = match msg.df {
            ExtendedSquitterADSB(msg) => match msg.message {
                ME::BDS05(me) => me,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        let Position {
            latitude,
            longitude,
        } = airborne_position_with_reference(&msg, 49.0, 6.0).unwrap();

        assert_relative_eq!(latitude, 49.81755, max_relative = 1e-3);
        assert_relative_eq!(longitude, 6.08442, max_relative = 1e-3);
    }

    #[test]
    fn decode_surface_position_with_reference() {
        let bytes = hex!("8c4841753aab238733c8cd4020b1");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();

        let msg = match msg.df {
            ExtendedSquitterADSB(msg) => match msg.message {
                ME::BDS06(me) => me,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        let Position {
            latitude,
            longitude,
        } = surface_position_with_reference(&msg, 51.99, 4.375).unwrap();

        assert_relative_eq!(latitude, 52.32061, max_relative = 1e-3);
        assert_relative_eq!(longitude, 4.73473, max_relative = 1e-3);
    }
}
