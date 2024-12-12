/**
 * Timestamps are encoded in various formats that may require some conversions.
 *
 * - Unix timestamps (from epoch), may be expressed in s, ms, us, ns
 *   This is provided by std::time::SystemTime
 * - nanoseconds since midnight UTC (Beast format)
 * - nanoseconds since GPT time of week
 */
use std::time::{SystemTime, UNIX_EPOCH};

static GPS_TO_UNIX_OFFSET: u64 = 315964800; // GPS epoch to Unix epoch in seconds

static LEAP_SECONDS_SINCE_2017: u64 = 18;

pub fn now_in_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before unix epoch")
        .as_nanos()
}

pub fn now_in_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before unix epoch")
        .as_secs()
}

pub fn today_in_s(now_s: u128) -> u128 {
    86_400 * (now_s / 86_400)
}

pub fn gps_week_in_s(now_s: u64) -> u64 {
    (86_400 * 7)
        * ((now_s - GPS_TO_UNIX_OFFSET + LEAP_SECONDS_SINCE_2017) / 86_400 / 7)
        + GPS_TO_UNIX_OFFSET
        - LEAP_SECONDS_SINCE_2017
}

pub fn since_today_to_nanos(nanos: u128) -> u128 {
    today_in_s(now_in_ns() / 1_000_000_000) * 1_000_000_000 + nanos
}

pub fn since_gps_week_to_since_today(gps_ns: u64) -> u64 {
    (gps_ns - LEAP_SECONDS_SINCE_2017 * 1_000_000_000) % 86_400_000_000_000
}

pub fn since_gps_week_to_unix_s(gps_ns: u64) -> f64 {
    gps_week_in_s(now_in_s()) as f64 + (gps_ns as f64 * 1e-9)
}
