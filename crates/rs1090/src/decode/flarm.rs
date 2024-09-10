use std::fmt;

use deku::prelude::*;
use serde::Serialize;

const KEY1: [i64; 4] = [0xe43276df, 0xdca83759, 0x9802b8ac, 0x4675a56b];
const KEY1B: [i64; 4] = [0xfc78ea65, 0x804b90ea, 0xb76542cd, 0x329dfa32];
const DELTA: u32 = 0x9E3779B9;

fn obscure(key: i64, seed: u64) -> i64 {
    let m1 = seed.wrapping_mul((key ^ (key >> 16)) as u64) as u32;
    let m2 = seed.wrapping_mul((m1 ^ (m1 >> 16)) as u64) as u32;
    (m2 ^ (m2 >> 16)) as i64
}

fn make_key(time: i64, address: i64) -> [u32; 4] {
    let table = if ((time >> 23) & 255) & 0x01 != 0 {
        &KEY1B
    } else {
        &KEY1
    };
    table.map(|tab| {
        let obs = obscure(tab ^ ((time >> 6) ^ address), 0x045D9F3B);
        (obs ^ 0x87B562F4) as u32
    })
}

fn mx(sum: u32, y: u32, z: u32, p: u32, e: u32, k: &[u32]) -> u32 {
    ((z >> 5 ^ y << 2).wrapping_add(y >> 3 ^ z << 4))
        ^ ((sum ^ y).wrapping_add(k[(p & 3 ^ e) as usize] ^ z))
}

fn fixk(k: &[u32]) -> Vec<u32> {
    let mut key = k.to_owned();
    if key.len() < 4 {
        let length = key.len();
        for _ in length..4 {
            key.push(0)
        }
    }
    key
}

fn btea(v: &mut [u32], k: &[u32]) {
    let length: u32 = v.len() as u32;
    let n: u32 = length - 1;
    let key: Vec<u32> = fixk(k);
    let mut e: u32;
    let mut y: u32 = v[0];
    let mut z;
    let q: u32 = 6; //+ 52 / length;
    let mut sum: u32 = q.wrapping_mul(DELTA);
    while sum != 0 {
        e = sum >> 2 & 3;
        let mut p: usize = n as usize;
        while p > 0 {
            z = v[p - 1];
            v[p] = v[p].wrapping_sub(mx(sum, y, z, p as u32, e, &key));
            y = v[p];
            p -= 1;
        }
        z = v[n as usize];
        v[0] = v[0].wrapping_sub(mx(sum, y, z, 0, e, &key));
        y = v[0];
        sum = sum.wrapping_sub(DELTA);
    }
}

/**
 * ## FLARM
 *
 * Swiss glider anti-colission system moved to a new encryption scheme, XXTEA.
 * The algorithm encrypts all the packet after the header: total 20 bytes or 5
 * long int words of data.
 *
 * XXTEA description and code are found here:
 * <http://en.wikipedia.org/wiki/XXTEA>. The system uses 6 iterations of the
 * main loop.
 *
 * The system version 6 sends two type of packets: position and ... some unknown
 * data. The difference is made by bit 0 of byte 3 of the packet: for position
 * data this bit is zero.
 *
 * For position data, the key used depends on the time and transmitting device
 * address. The key is as well obscured by a weird algorithm.
 *
 * Reference: <https://pastebin.com/YK2f8bfm>
 *
 * Decode with [`Flarm::from_record`]
 */

/*
 * NEW PACKET FORMAT:
 *
 * Byte     Bits
 *  0     AAAA AAAA    device address
 *  1     AAAA AAAA
 *  2     AAAA AAAA
 *  3     00aa 0000    aa = 10 or 01
 *
 * (encrypted below)
 *  4     vvvv vvvv    vertical speed
 *  5     xxxx xxvv
 *  6     gggg gggg    GPS status
 *  7     tttt gggg    plane type
 *
 *  8     LLLL LLLL    Latitude
 *  9     LLLL LLLL
 * 10     aaaa aLLL
 * 11     aaaa aaaa    Altitude
 *
 * 12     NNNN NNNN    Longitude
 * 13     NNNN NNNN
 * 14     xxxx NNNN
 * 15     FFxx xxxx    multiplying factor
 *
 * 16     SSSS SSSS    n/s derivatives
 * 17     ssss ssss
 * 18     KKKK KKKK
 * 19     kkkk kkkk
 *
 * 20     EEEE EEEE    e/w derivatives
 * 21     eeee eeee
 * 22     PPPP PPPP
 * 24     pppp pppp
 *
 */

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
pub struct Flarm {
    #[deku(endian = "little")]
    /// The timestamp must be passed for the decoding
    pub timestamp: u32,

    /// A reference latitude, usually the latitude of the receiver
    pub reference_lat: f64,

    /// A reference longitude, usually the longitude of the receiver
    pub reference_lon: f64,

    /// The address of the transmitting device, either an icao24 address, or not
    pub icao24: Address,

    #[deku(map = "|v| -> Result<_, DekuError> { magic_value(v) }")]
    /// A flag set to true if the address is an icao24 address
    pub is_icao24: bool,

    #[deku(reader = "Self::decode_btea(deku::reader, *icao24, *timestamp)")]
    #[serde(skip)]
    /// Decrypted information (as a table 5 u32 integers)
    pub decoded: Vec<u32>,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {
        Ok(1 << ((decoded[2] >> 30) & 30) & 0x3)
    }"
    )]
    #[serde(skip)]
    /// A multiplying factor used for decoding vertical and lateral speeds
    pub mult: i32,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {Self::decode_actype(decoded[0])}"
    )]
    /// A enum describing the type of aircraft (glider, paraglider, UAV, etc.)
    pub actype: AircraftType,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {
            Self::decode_latitude(decoded[1], *reference_lat)
        }"
    )]
    /// Decoded latitude in degrees
    pub latitude: f64,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {
            Self::decode_longitude(decoded[2], *reference_lon)
        }"
    )]
    /// Decoded longitude in degrees
    pub longitude: f64,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> { Ok((decoded[1]>>19) & 0x1fff) }"
    )]
    /// GPS altitude in meters
    pub geoaltitude: u32,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {
            Ok((((decoded[0] & 0x3ff) as i8) as i32 * mult) as f64 / 10.)
        }"
    )]
    /// Vertical speed in meters/second
    pub vertical_speed: f64,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {
            Ok((0..4).map(|i| (((decoded[3] >> (i * 8)) & 0xFF) as i8) as i32 * mult).collect())
        }"
    )]
    #[serde(skip)]
    /// Derivative along the North/South axis, at several timestamps ahead
    pub ns: Vec<i32>,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {
            Ok((0..4).map(|i| (((decoded[4] >> (i * 8)) & 0xFF) as i8) as i32 * mult).collect())
        }"
    )]
    #[serde(skip)]
    /// Derivative along the East/West axis, at several timestamps ahead
    pub ew: Vec<i32>,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {Self::decode_groundspeed(ns, ew)}"
    )]
    /// An estimation of the groundspeed in meters/second
    pub groundspeed: f64,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> {Self::decode_track(ns, ew, *groundspeed)}"
    )]
    /// An estimation of the track angle in degrees
    pub track: f64,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> { Ok(((decoded[0] >> 14) & 0x1) == 1) }"
    )]
    /// A flag set to true if the transmitter asks to be tracked
    pub no_track: bool,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> { Ok(((decoded[0] >> 13) & 0x1) == 1) }"
    )]
    /// A flag set to true if the transmitter is in stealth mode
    pub stealth: bool,

    #[deku(
        bits = 1,
        map = "|_v: bool| -> Result<_, DekuError> { Ok((decoded[0] >> 16) & 0xFFF) }"
    )]
    pub gps: u32,
}

#[derive(Debug, PartialEq, DekuRead, Clone, Copy)]
pub struct Address(#[deku(endian = "little", bits = "24")] u32);

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:06x}", self.0)?;
        Ok(())
    }
}

impl serde::ser::Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let icao = format!("{:06x}", &self.0);
        serializer.serialize_str(&icao)
    }
}

fn magic_value(v: u8) -> Result<bool, DekuError> {
    if v == 0x10 {
        return Ok(true);
    };
    if v == 0x20 {
        return Ok(false);
    };
    Err(DekuError::Assertion("Magic must be 0x10 or 0x20".into()))
}

#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[deku(id_type = "u8", bits = "4", endian = "big")]
pub enum AircraftType {
    Unknown = 0,
    Glider,
    Towplane,
    Helicopter,
    Parachute,
    DropPlane,
    Hangglider,
    Paraglider,
    Aircraft,
    Jet,
    UFO,
    Balloon,
    Airship,
    UAV,
    Reserved,
    StaticObstacle,
}

impl Flarm {
    fn decode_btea<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
        reader: &mut Reader<R>,
        icao24: Address,
        timestamp: u32,
    ) -> Result<Vec<u32>, DekuError> {
        let addr = (icao24.0 << 8) & 0xffffff;
        let key = make_key(timestamp as i64, addr as i64);

        let p1 = u32::from_reader_with_ctx(reader, deku::ctx::Endian::Little)?;
        let p2 = u32::from_reader_with_ctx(reader, deku::ctx::Endian::Little)?;
        let p3 = u32::from_reader_with_ctx(reader, deku::ctx::Endian::Little)?;
        let p4 = u32::from_reader_with_ctx(reader, deku::ctx::Endian::Little)?;
        let p5 = u32::from_reader_with_ctx(reader, deku::ctx::Endian::Little)?;
        let mut decoded = vec![p1, p2, p3, p4, p5];
        btea(&mut decoded, key.as_ref());

        Ok(decoded)
    }

    fn decode_latitude(decoded: u32, ref_lat: f64) -> Result<f64, DekuError> {
        let mut lat = (decoded & 0x7FFFF) as i32;
        let round_lat = ((ref_lat * 1e7) as i32) >> 7;
        lat = (lat - round_lat).rem_euclid(0x080000);
        if lat >= 0x040000 {
            lat -= 0x080000;
        }
        Ok((((lat + round_lat) << 7) + 0x40) as f64 * 1e-7)
    }

    fn decode_longitude(decoded: u32, ref_lon: f64) -> Result<f64, DekuError> {
        let mut lon = (decoded & 0xFFFFF) as i32;
        let round_lon = ((ref_lon * 1e7) as i32) >> 7;
        lon = (lon - round_lon).rem_euclid(0x100000);
        if lon >= 0x080000 {
            lon -= 0x100000;
        }
        Ok((((lon + round_lon) << 7) + 0x40) as f64 * 1e-7)
    }

    fn decode_actype(decoded: u32) -> Result<AircraftType, DekuError> {
        let ac = match (decoded >> 28) & 0xf {
            0 => AircraftType::Unknown,
            1 => AircraftType::Glider,
            2 => AircraftType::Towplane,
            3 => AircraftType::Helicopter,
            4 => AircraftType::Parachute,
            5 => AircraftType::DropPlane,
            6 => AircraftType::Hangglider,
            7 => AircraftType::Paraglider,
            8 => AircraftType::Aircraft,
            9 => AircraftType::Jet,
            10 => AircraftType::UFO,
            11 => AircraftType::Balloon,
            12 => AircraftType::Airship,
            13 => AircraftType::UAV,
            14 => AircraftType::Reserved,
            15 => AircraftType::StaticObstacle,
            _ => unreachable!(),
        };
        Ok(ac)
    }

    fn decode_groundspeed(ns: &[i32], ew: &[i32]) -> Result<f64, DekuError> {
        let speed: f64 = ns
            .iter()
            .zip(ew.iter())
            .map(|(&n, &e)| {
                let ns = n as f64 / 4.0;
                let ew = e as f64 / 4.0;
                (ns * ns + ew * ew).sqrt()
            })
            .sum::<f64>();
        Ok(speed / 4.0)
    }

    fn decode_track(ns: &[i32], ew: &[i32], v: f64) -> Result<f64, DekuError> {
        let track = |y: f64, x: f64| {
            let v = if v < 1e-6 { 1. } else { v };
            (libm::atan2(x / v / 4., y / v / 4.) / 0.01745).rem_euclid(360.)
        };
        let track4 = track(ns[0] as f64, ew[0] as f64);
        let track8 = track(ns[1] as f64, ew[1] as f64);

        let turning_rate =
            |a1: f64, a2: f64| ((a2 - a1) + 540.).rem_euclid(360.) - 180.;

        let track = track4 - 4. * turning_rate(track4, track8) / 4.;

        Ok(track)
    }

    /**
     * The method wraps the [`Flarm::from_bytes`] used to decode deku structures.
     *
     * Parameters include:
     * - the `timestamp` is necessary to decrypt the message;
     * - the `reference` position (lat/lon in degrees);
     * - the actual message as a u8 slice reference.
     */
    pub fn from_record(
        timestamp: u32,
        reference: &[f64; 2],
        msg: &[u8],
    ) -> Result<Self, DekuError> {
        let mut combined_bytes = Vec::new();
        combined_bytes.extend_from_slice(&timestamp.to_le_bytes());
        combined_bytes.extend_from_slice(&reference[0].to_ne_bytes());
        combined_bytes.extend_from_slice(&reference[1].to_ne_bytes());
        combined_bytes.extend_from_slice(msg);

        match Flarm::from_bytes((&combined_bytes, 0)) {
            Ok((_, flarm)) => Ok(flarm),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_flarm() {
        let msg = hex!("7bf23810860b7eabb23952252fd4927024b21fd94e9e1ef416f0");
        let latlon: [f64; 2] = [43.61924, 5.11755];
        let ts = 1655274034_u32;

        let flarm = Flarm::from_record(ts, &latlon, &msg).unwrap();
        //println!("{}", serde_json::to_string(&flarm).unwrap());

        assert!(flarm.icao24.0 == 0x38f27b);
        assert!(flarm.is_icao24);
        assert!(flarm.actype == AircraftType::Glider);
        assert_relative_eq!(flarm.latitude, 43.61822, max_relative = 1e-3);
        assert_relative_eq!(flarm.longitude, 5.117242, max_relative = 1e-3);
        assert!(flarm.geoaltitude == 160);
        assert_relative_eq!(flarm.vertical_speed, -1.1, max_relative = 1e-3);
        assert_relative_eq!(flarm.groundspeed, 0.7905694, max_relative = 1e-3);
        assert_relative_eq!(flarm.track, 198.40446, max_relative = 1e-3);
        assert!(!flarm.no_track);
        assert!(!flarm.stealth);
        assert!(flarm.gps == 3926);

        let msg = hex!("7bf2381040ccc7e2395ecaa28e033a655d47e1d91d0bf986e1b0");
        let ts = 1655279476_u32;

        let flarm = Flarm::from_record(ts, &latlon, &msg).unwrap();
        //println!("{}", serde_json::to_string(&flarm).unwrap());

        assert_relative_eq!(flarm.latitude, 43.68129, max_relative = 1e-3);
        assert_relative_eq!(flarm.longitude, 5.15059, max_relative = 1e-3);
    }
}
