use deku::prelude::*;
use serde::Serialize;

/**
 * ## Track and turn report (BDS 5,0)
 */
#[derive(Debug, PartialEq, Serialize, DekuRead, Clone)]
#[serde(tag = "bds", rename = "50")]
pub struct TrackAndTurnReport {
    #[deku(reader = "read_roll(deku::reader)")] // 11 bits
    #[serde(rename = "roll")]
    // Roll angle (negative sign means left wing down)
    pub roll_angle: Option<f64>,

    #[deku(reader = "read_track(deku::reader)")] // 12 bits
    #[serde(rename = "track")]
    pub track_angle: Option<f64>,

    #[deku(reader = "read_groundspeed(deku::reader)")] // 11 bits
    /// Groundspeed in kts
    pub groundspeed: Option<u16>,

    #[deku(reader = "read_rate(deku::reader, *roll_angle)")] // 11 bits
    pub track_rate: Option<f64>,

    #[deku(reader = "read_tas(deku::reader, *groundspeed)")] // 11 bits
    #[serde(rename = "TAS")]
    /// True Airspeed (TAS) in kts, IAS is in BDS 0,6
    pub true_airspeed: Option<u16>,
}

fn read_roll<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<f64>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let sign = u8::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(9)),
    )?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion(
                "Non-null value with invalid status: roll angle".into(),
            ));
        } else {
            return Ok(None);
        }
    }

    let roll = if sign > 0 {
        (value as f64 - 512.) * 45. / 256.
    } else {
        value as f64 * 45. / 256.
    };
    if roll.abs() > 50. {
        return Err(DekuError::Assertion(
            format!("Roll angle: abs({}) > 50", roll).into(),
        ));
    }
    Ok(Some(roll))
}

fn read_track<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<f64>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let sign = u8::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(10)),
    )?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion(
                "Non-null value with invalid status: track angle".into(),
            ));
        } else {
            return Ok(None);
        }
    }

    let value = if sign == 1 {
        value as i16 - 1024
    } else {
        value as i16
    };
    let mut track = value as f64 * 90. / 512.;
    if track < 0. {
        track += 360.
    }

    Ok(Some(track))
}

fn read_groundspeed<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<u16>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(10)),
    )?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion(
                "Non-null value with invalid status: groundspeed".into(),
            ));
        } else {
            return Ok(None);
        }
    }

    let gs = value * 2;
    if gs > 600 {
        return Err(DekuError::Assertion(
            format!("Groundspeed value: {} > 600", gs).into(),
        ));
    }
    Ok(Some(gs))
}

fn read_rate<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
    roll: Option<f64>,
) -> Result<Option<f64>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let sign = u8::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(9)),
    )?;

    if !status {
        if (sign != 0) | (value != 0) {
            return Err(DekuError::Assertion(
                "Non-null value with invalid status: track rate".into(),
            ));
        } else {
            return Ok(None);
        }
    }

    if value == 0b111111111 {
        return Ok(None);
    }

    let value = if sign == 1 {
        value as i16 - 512
    } else {
        value as i16
    };
    let rate = value as f64 * 8. / 256.;

    if let Some(roll) = roll {
        if roll * rate < 0. {
            // signs must agree: left wing down = turn left
            return Err(DekuError::Assertion(
                format!(
                    "Roll angle {} and track rate {} signs do not agree.",
                    roll, rate
                )
                .into(),
            ));
        }
    }

    Ok(Some(rate))
}

fn read_tas<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
    reader: &mut Reader<R>,
    gs: Option<u16>,
) -> Result<Option<u16>, DekuError> {
    let status = bool::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(1)),
    )?;
    let value = u16::from_reader_with_ctx(
        reader,
        (deku::ctx::Endian::Big, deku::ctx::BitSize(10)),
    )?;

    if !status {
        if value != 0 {
            return Err(DekuError::Assertion(
                "Non-null value with invalid status: true air speed".into(),
            ));
        } else {
            return Ok(None);
        }
    }

    let tas = value * 2;

    if let Some(gs) = gs {
        if !(80..=500).contains(&tas) | ((gs as i16 - tas as i16).abs() > 200) {
            return Err(DekuError::Assertion(format!(
                "TAS = {} must be within [80, 500] and abs(GS - TAS) = {} < 200",
                tas, gs
            ).into()));
        }
    }
    Ok(Some(tas))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use approx::assert_relative_eq;
    use hexlit::hex;

    #[test]
    fn test_valid_bds50() {
        let bytes = hex!("a000139381951536e024d4ccf6b5");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let CommBAltitudeReply { bds, .. } = msg.df {
            let TrackAndTurnReport {
                roll_angle,
                track_angle,
                true_airspeed,
                groundspeed,
                track_rate,
            } = bds.bds50.unwrap();
            assert_relative_eq!(roll_angle.unwrap(), 2.1, max_relative = 1e-2);
            assert_relative_eq!(
                track_angle.unwrap(),
                114.258,
                max_relative = 1e-3
            );
            assert_eq!(groundspeed.unwrap(), 438);
            assert_eq!(true_airspeed.unwrap(), 424);
            assert_relative_eq!(
                track_rate.unwrap(),
                0.125,
                max_relative = 1e-3
            );
        } else {
            unreachable!();
        }
    }
    #[test]
    fn test_invalid_bds50() {
        let bytes = hex!("a0000638fa81c10000000081a92f");
        let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
        if let CommBAltitudeReply { bds, .. } = msg.df {
            assert_eq!(bds.bds50, None);
        } else {
            unreachable!();
        }
    }
}
