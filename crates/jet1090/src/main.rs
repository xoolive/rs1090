#![doc = include_str!("../readme.md")]

mod aircraftdb;
mod cli;
mod snapshot;
mod table;
mod tui;
mod web;

use clap::Parser;
use crossterm::event::KeyCode;
use ratatui::widgets::*;
use rs1090::decode::cpr::{decode_position, AircraftState, Position};
use rs1090::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tui::Event;
use warp::Filter;
use web::TrackQuery;

#[derive(Debug, Parser)]
#[command(
    name = "jet1090",
    version,
    author = "xoolive",
    about = "Decode and serve Mode S demodulated raw messages"
)]
struct Options {
    /// Activate JSON output
    #[arg(short, long, default_value = "false")]
    verbose: bool,

    /// Dump a copy of the received messages as .jsonl
    #[arg(short, long, default_value=None)]
    output: Option<String>,

    /// Display a table in interactive mode (not compatible with verbose)
    #[arg(short, long, default_value = "false")]
    interactive: bool,

    /// Port for the API endpoint (on 0.0.0.0)
    #[arg(long, default_value=None)]
    serve_port: Option<u16>,

    /// How much history to expire (in minutes)
    #[arg(long, short = 'x')]
    expire: Option<u64>,

    /// List the sources of data following the format \[host:\]port\[\@reference\]
    //
    // - `host` can be a DNS name, an IP address or `rtlsdr` (for RTL-SDR dongles)
    // - `port` must be a number
    // - `reference` can be LFPG for major airports, `43.3,1.35` otherwise
    sources: Vec<cli::Source>,
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

    let aircraftdb = aircraftdb::aircraft().await;

    let mut aircraft: BTreeMap<ICAO, AircraftState> = BTreeMap::new();

    let terminal = if options.interactive {
        Some(tui::init()?)
    } else {
        None
    };
    let width = if let Some(terminal) = &terminal {
        terminal.size()?.width
    } else {
        0
    };

    let mut events = tui::EventHandler::new(width);

    let app_tui = Arc::new(Mutex::new(Jet1090 {
        items: Vec::new(),
        state: TableState::default().with_selected(0),
        scroll_state: ScrollbarState::new(0),
        should_quit: false,
        state_vectors: BTreeMap::new(),
        sort_key: SortKey::default(),
        sort_asc: true,
        width,
    }));
    let app_dec = app_tui.clone();
    let app_web = app_tui.clone();
    let app_exp = app_tui.clone();

    if let Some(mut terminal) = terminal {
        tokio::spawn(async move {
            loop {
                if let Ok(event) = events.next().await {
                    let _ = update(&mut app_tui.lock().await, event);
                }
                let mut app = app_tui.lock().await;
                if app.should_quit {
                    break;
                }
                terminal.draw(|frame| table::build_table(frame, &mut app))?;
            }
            tui::restore()
        });
    }

    if let Some(minutes) = options.expire {
        tokio::spawn(async move {
            let app_expire = app_exp.clone();
            loop {
                sleep(Duration::from_secs(60)).await;
                {
                    let mut app = app_expire.lock().await;
                    let now = SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("SystemTime before unix epoch")
                        .as_secs();

                    let remove_keys = app
                        .state_vectors
                        .iter()
                        .filter(|(_key, value)| {
                            now > value.cur.last + minutes * 60
                        })
                        .map(|(key, _)| key.to_string())
                        .collect::<Vec<String>>();

                    for key in remove_keys {
                        app.state_vectors.remove(&key);
                    }

                    let _ = app
                        .state_vectors
                        .iter_mut()
                        .map(|(_key, value)| {
                            value.hist.retain(|elt| {
                                now < (elt.timestamp as u64) + minutes * 60
                            })
                        })
                        .collect::<Vec<()>>();
                }
            }
        });
    }

    if let Some(port) = options.serve_port {
        tokio::spawn(async move {
            let app_home = app_web.clone();
            let home = warp::path::end()
                .and(warp::any().map(move || app_home.clone()))
                .and_then(|app: Arc<Mutex<Jet1090>>| async move {
                    web::icao24(&app).await
                });

            let app_all = app_web.clone();
            let all = warp::path("all")
                .and(warp::any().map(move || app_all.clone()))
                .and_then(|app: Arc<Mutex<Jet1090>>| async move {
                    web::all(&app).await
                });

            let app_track = app_web.clone();
            let track = warp::get()
                .and(warp::path("track"))
                .and(warp::any().map(move || app_track.clone()))
                .and(warp::query::<TrackQuery>())
                .and_then(
                    |app: Arc<Mutex<Jet1090>>, q: TrackQuery| async move {
                        web::track(&app, q).await
                    },
                );

            let cors = warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["*"])
                .allow_methods(vec!["GET"]);

            let routes = warp::get()
                .and(home.or(all).or(track))
                .recover(web::handle_rejection)
                .with(cors);

            warp::serve(routes).run(([0, 0, 0, 0], port)).await;
        });
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let references = options
        .sources
        .iter()
        .map(|s| s.reference)
        .collect::<Vec<Option<Position>>>();

    for (idx, source) in options.sources.into_iter().enumerate() {
        let tx_copy = tx.clone();
        tokio::spawn(async move {
            source.receiver(tx_copy, idx).await;
        });
    }

    while let Some(tmsg) = rx.recv().await {
        let frame = hex::decode(&tmsg.frame).unwrap();
        if let Ok((_, msg)) = Message::from_bytes((&frame, 0)) {
            let mut msg = TimedMessage {
                timestamp: tmsg.timestamp,
                frame: tmsg.frame.to_string(),
                message: Some(msg),
                idx: tmsg.idx,
            };
            let mut reference = references[tmsg.idx];

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

            snapshot::update_snapshot(&app_dec, &mut msg, &aircraftdb).await;

            if let Ok(json) = serde_json::to_string(&msg) {
                if options.verbose {
                    println!("{}", json);
                }
                if let Some(file) = &mut file {
                    file.write_all(json.as_bytes()).await?;
                    file.write_all("\n".as_bytes()).await?;
                }
            }

            snapshot::store_history(&app_dec, msg, &aircraftdb).await;
        }
        if app_dec.lock().await.should_quit {
            break;
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct Jet1090 {
    state: TableState,
    items: Vec<String>,
    scroll_state: ScrollbarState,
    should_quit: bool,
    state_vectors: BTreeMap<String, snapshot::StateVectors>,
    sort_key: SortKey,
    sort_asc: bool,
    width: u16,
}

#[derive(Debug, Default, PartialEq)]
pub enum SortKey {
    CALLSIGN,
    #[default]
    ALTITUDE,
    VRATE,
    FIRST,
    LAST,
}

fn update(
    jet1090: &mut tokio::sync::MutexGuard<Jet1090>,
    event: Event,
) -> std::io::Result<()> {
    match event {
        Event::Key(key) => {
            use KeyCode::*;
            match key.code {
                Char('j') | Down => jet1090.next(),
                Char('k') | Up => jet1090.previous(),
                Char('g') | PageUp | Home => jet1090.home(),
                Char('q') | Esc => jet1090.should_quit = true,
                Char('a') => {
                    jet1090.sort_key = SortKey::ALTITUDE;
                }
                Char('c') => {
                    jet1090.sort_key = SortKey::CALLSIGN;
                }
                Char('v') => {
                    jet1090.sort_key = SortKey::VRATE;
                }
                Char('f') => {
                    jet1090.sort_key = SortKey::FIRST;
                }
                Char('l') => {
                    jet1090.sort_key = SortKey::LAST;
                }
                Char('-') => jet1090.sort_asc = !jet1090.sort_asc,
                _ => {}
            }
        }
        Event::Tick(size) => jet1090.width = size,
        _ => {}
    }
    Ok(())
}

impl Jet1090 {
    pub fn keys(&self) -> Result<impl warp::Reply, std::convert::Infallible> {
        let keys: Vec<_> = self
            .state_vectors
            .keys()
            .map(|key| key.to_string())
            .collect();
        Ok(warp::reply::json(&keys))
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }
    pub fn home(&mut self) {
        self.state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }
}
