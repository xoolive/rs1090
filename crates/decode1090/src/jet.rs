#![doc = include_str!("../readme.md")]

mod tui;

use chrono::prelude::*;
use clap::Parser;
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};
use rs1090::decode::adsb::{ADSB, ME};
use rs1090::decode::bds::bds09::AirborneVelocitySubType::AirspeedSubsonic;
use rs1090::decode::bds::bds09::AirborneVelocitySubType::GroundSpeedDecoding;
use rs1090::decode::bds::bds09::AirspeedType::{IAS, TAS};
use rs1090::decode::cpr::{decode_position, AircraftState, Position};
use rs1090::decode::IdentityCode;
use rs1090::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tui::Event;

#[derive(Debug, Parser)]
#[command(
    name = "jet1090",
    version,
    author = "xoolive",
    about = "Decode Mode S demodulated raw messages"
)]
struct Options {
    /// Address of the demodulating server (beast feed)
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port of the demodulating server
    #[arg(short, long, default_value = None)]
    port: Option<u16>,

    /// Demodulate data from a RTL-SDR dongle
    #[arg(long, default_value = "false")]
    rtlsdr: bool,

    /// Activate JSON output
    #[arg(short, long, default_value = "false")]
    verbose: bool,

    /// Dump a copy of the received messages as .jsonl
    #[arg(short, long, default_value=None)]
    output: Option<String>,

    /// Reference coordinates for the decoding (e.g.
    //  --latlon LFPG for major airports,
    /// --latlon 43.3,1.35 or --latlon ' -34,18.6' if negative)
    #[arg(long, default_value=None)]
    latlon: Option<Position>,

    /// Display a table in interactive mode (not compatible with verbose)
    #[arg(short, long, default_value = "false")]
    interactive: bool,

    /// How to serve the collected data (todo!())
    #[arg(long, default_value=None)]
    serve: Option<u8>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse();

    let mut file = if let Some(output_path) = options.output {
        Some(
            fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(output_path)
                .await?,
        )
    } else {
        None
    };

    let mut reference = options.latlon;
    let mut aircraft: BTreeMap<ICAO, AircraftState> = BTreeMap::new();

    let states: Arc<Mutex<BTreeMap<String, StateVectors>>> =
        Arc::new(Mutex::new(BTreeMap::new()));
    let states_tui = Arc::clone(&states);

    let mut rx = if options.rtlsdr {
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
            rtlsdr::receiver().await
        }
    } else {
        let server_address =
            format!("{}:{}", options.host, options.port.unwrap());
        radarcape::receiver(server_address).await
    };

    let mut terminal = tui::init()?;
    let mut events = tui::EventHandler::new();
    let mut app = App {
        counter: 0,
        should_quit: false,
    };

    tokio::spawn(async move {
        loop {
            if let Ok(event) = events.next().await {
                let _ = update(&mut app, event);
            } // new
            if app.should_quit {
                break;
            }
            terminal.draw(|frame| build_table(frame, &states_tui))?;
        }
        tui::restore()
    });

    while let Some(tmsg) = rx.recv().await {
        let frame = hex::decode(&tmsg.frame).unwrap();
        if let Ok((_, msg)) = Message::from_bytes((&frame, 0)) {
            let mut msg = TimedMessage {
                timestamp: tmsg.timestamp,
                frame: tmsg.frame.to_string(),
                message: Some(msg),
            };

            if let Some(message) = &mut msg.message {
                match &mut message.df {
                    ExtendedSquitterADSB(adsb) => decode_position(
                        &mut adsb.message,
                        msg.timestamp,
                        &adsb.icao24,
                        &mut aircraft,
                        &mut reference,
                    ),
                    ExtendedSquitterTisB { cf, .. } => decode_position(
                        &mut cf.me,
                        msg.timestamp,
                        &cf.aa,
                        &mut aircraft,
                        &mut reference,
                    ),
                    _ => {}
                }
            };

            update_snapshot(&states, &mut msg).await;

            let json = serde_json::to_string(&msg).unwrap();
            if options.verbose {
                println!("{}", json);
            }
            if let Some(file) = &mut file {
                file.write_all(json.as_bytes()).await?;
                file.write_all("\n".as_bytes()).await?;
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct StateVectors {
    pub cur: Snapshot,
    //pub hist: Vec<TimedMessage>,
}

impl StateVectors {
    fn new(ts: u64, icao24: String) -> StateVectors {
        let cur = Snapshot {
            icao24,
            first: ts,
            last: ts,
            callsign: None,
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
            selected_heading: None,
            nic: None,
        };
        StateVectors { cur }
    }
}

#[derive(Debug)]
pub struct Snapshot {
    pub icao24: String,
    pub first: u64,
    pub last: u64,
    pub callsign: Option<String>,
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
    pub selected_heading: Option<f32>,
    pub nic: Option<u8>,
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

async fn update_snapshot(
    states: &Mutex<BTreeMap<String, StateVectors>>,
    msg: &mut TimedMessage,
) {
    if let TimedMessage {
        timestamp,
        message: Some(message),
        ..
    } = msg
    {
        if let Some(icao24) = icao24(message) {
            let mut states = states.lock().unwrap();
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
                        aircraft.cur.selected_heading = bds62.selected_heading;
                    }
                    _ => {}
                },
                ExtendedSquitterTisB { cf, .. } => match &cf.me {
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
                    }
                    ME::BDS08(bds08) => {
                        aircraft.cur.callsign = Some(bds08.callsign.to_string())
                    }
                    _ => {}
                },
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
                    }
                }
                _ => {}
            };
        }
    }
}

fn ts_to_utc(timestamp: u64) -> String {
    let dt: DateTime<Utc> =
        DateTime::from_timestamp(timestamp as i64, 0).unwrap();
    format!("{}", dt.format("%H:%M:%S"))
}

fn build_table(
    frame: &mut Frame<'_>,
    states_tui: &Arc<Mutex<BTreeMap<String, StateVectors>>>,
) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before unix epoch")
        .as_secs();

    let rows: Vec<Row> = states_tui
        .lock()
        .unwrap()
        .values()
        .filter(|sv| (now as i64 - sv.cur.last as i64) < 30)
        .map(|sv| {
            Row::new(vec![
                sv.cur.icao24.to_owned(),
                sv.cur.callsign.to_owned().unwrap_or("".to_string()),
                sv.cur
                    .squawk
                    .map(|s| s.to_string())
                    .unwrap_or("".to_string()),
                if let Some(lat) = sv.cur.latitude {
                    format!("{}", lat)
                } else {
                    "".to_string()
                },
                if let Some(lon) = sv.cur.longitude {
                    format!("{}", lon)
                } else {
                    "".to_string()
                },
                if let Some(alt) = sv.cur.altitude {
                    format!("{}", alt)
                } else {
                    "".to_string()
                },
                if let Some(sel) = sv.cur.selected_altitude {
                    format!("{}", sel)
                } else {
                    "".to_string()
                },
                if let Some(gs) = sv.cur.groundspeed {
                    format!("{}", gs)
                } else {
                    "".to_string()
                },
                if let Some(tas) = sv.cur.tas {
                    format!("{}", tas)
                } else {
                    "".to_string()
                },
                if let Some(ias) = sv.cur.ias {
                    format!("{}", ias)
                } else {
                    "".to_string()
                },
                if let Some(mach) = sv.cur.mach {
                    format!("{}", mach)
                } else {
                    "".to_string()
                },
                if let Some(vrate) = sv.cur.vertical_rate {
                    format!("{}", vrate)
                } else {
                    "".to_string()
                },
                if let Some(trk) = sv.cur.track {
                    format!("{}", trk)
                } else {
                    "".to_string()
                },
                if let Some(heading) = sv.cur.heading {
                    format!("{}", heading)
                } else {
                    "".to_string()
                },
                if let Some(selected_heading) = sv.cur.selected_heading {
                    format!("{}", selected_heading)
                } else {
                    "".to_string()
                },
                if let Some(roll) = sv.cur.roll {
                    format!("{}", roll)
                } else {
                    "".to_string()
                },
                if let Some(nic) = sv.cur.nic {
                    format!("{}", nic)
                } else {
                    "".to_string()
                },
                if now > sv.cur.last {
                    format!("{}s ago", now - sv.cur.last)
                } else {
                    "".to_string()
                },
                ts_to_utc(sv.cur.first),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(4),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Length(8),
    ];
    let size = &rows.len();
    let table = Table::new(rows, widths)
        .column_spacing(2)
        .header(
            Row::new(vec![
                "icao24", "callsign", "sqwk", "lat", "lon", "alt", "sel", "gs",
                "tas", "ias", "mach", "vrate", "trk", "hdg", "sel", "roll",
                "nic", "last", "first",
            ])
            .bottom_margin(1)
            .style(Style::new().bold()),
        )
        .block(
            Block::default()
                .title_bottom(format!("jet1090 ({} aircraft)", size))
                .title_alignment(Alignment::Right)
                .title_style(Style::new().blue().bold())
                .padding(Padding::symmetric(1, 0))
                .borders(Borders::ALL),
        )
        // The selected row and its content can also be styled.
        .highlight_style(Style::new().reversed())
        // ...and potentially show a symbol in front of the selection.
        .highlight_symbol(">>");

    frame.render_widget(table, frame.size());
}

#[derive(Debug, Default)]
pub struct App {
    counter: u8,
    should_quit: bool,
}

fn update(app: &mut App, event: Event) -> Result<()> {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('j') => app.counter += 1,
            KeyCode::Char('k') => app.counter -= 1,
            KeyCode::Char('q') => app.should_quit = true,
            _ => {}
        }
    }
    Ok(())
}
