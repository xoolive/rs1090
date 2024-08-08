#![doc = include_str!("../readme.md")]

mod aircraftdb;
mod cli;
mod snapshot;
mod table;
mod tui;
mod web;

use clap::{Command, CommandFactory, Parser, ValueHint};
use clap_complete::{generate, Generator, Shell};
use cli::Source;
use crossterm::event::KeyCode;
use ratatui::widgets::*;
use rs1090::decode::cpr::{decode_position, AircraftState};
use rs1090::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tui::Event;
use warp::Filter;
use web::TrackQuery;

#[derive(Default, Deserialize, Parser)]
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
    #[arg(short, long, default_value=None, value_hint=ValueHint::FilePath)]
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

    /// Should we update the reference positions (if the receiver is moving)
    #[arg(short, long, default_value = "false")]
    update_position: bool,

    /// Shell completion generation
    #[arg(long = "completion", value_enum)]
    #[serde(skip)]
    completion: Option<Shell>,

    /// List the sources of data following the format \[host:\]port\[\@reference\]
    //
    // - `host` can be a DNS name, an IP address or `rtlsdr` (for RTL-SDR dongles)
    // - `port` must be a number
    // - `reference` can be LFPG for major airports, `43.3,1.35` otherwise
    sources: Vec<cli::Source>,

    #[arg(short, long, value_name = "FILE", default_value = "jet1090.log")]
    log_file: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from a .env file
    dotenv::dotenv().ok();

    let mut options = Options::default();

    let mut cfg_path = dirs::config_dir().unwrap_or_default();
    cfg_path.push("jet1090");
    cfg_path.push("config.toml");

    if cfg_path.exists() {
        let string = fs::read_to_string(cfg_path).await.ok().unwrap();
        options = toml::from_str(&string).unwrap();
    }

    if let Ok(config_file) = std::env::var("JET1090_CONFIG") {
        let string = fs::read_to_string(config_file).await.ok().unwrap();
        options = toml::from_str(&string).unwrap();
    }

    let mut cli_options = Options::parse();

    // Generate completion instructions
    if let Some(generator) = cli_options.completion {
        let mut cmd = Options::command();
        print_completions(generator, &mut cmd);
        return Ok(());
    }

    if cli_options.verbose {
        options.verbose = true;
    }
    if cli_options.output.is_some() {
        options.output = cli_options.output;
    }
    if cli_options.interactive {
        options.interactive = true;
    }
    if cli_options.serve_port.is_some() {
        options.serve_port = cli_options.serve_port;
    }
    if cli_options.expire.is_some() {
        options.expire = cli_options.expire;
    }
    if cli_options.update_position {
        options.update_position = cli_options.update_position;
    }
    options.sources.append(&mut cli_options.sources);

    let log_file = std::fs::File::create(&cli_options.log_file)
        .expect("fail to create log file");
    let log_file_layer = fmt::layer().with_writer(log_file).with_ansi(false);

    // example: RUST_LOG=rs1090=DEBUG
    let env_filter = EnvFilter::from_default_env();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(log_file_layer)
        .init();

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
        sources: options.sources.clone(),
        items: Vec::new(),
        state: TableState::default().with_selected(0),
        scroll_state: ScrollbarState::new(0),
        should_quit: false,
        state_vectors: BTreeMap::new(),
        sort_key: SortKey::default(),
        sort_asc: false,
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

            let app_receivers = app_web.clone();
            let receivers = warp::path("receivers")
                .and(warp::any().map(move || app_receivers.clone()))
                .and_then(|app: Arc<Mutex<Jet1090>>| async move {
                    web::receivers(&app).await
                });

            let cors = warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["*"])
                .allow_methods(vec!["GET"]);

            let routes = warp::get()
                .and(home.or(all).or(track).or(receivers))
                .recover(web::handle_rejection)
                .with(cors);

            warp::serve(routes).run(([0, 0, 0, 0], port)).await;
        });
    }

    // I am not sure whether this size calibration is relevant, but let's try...
    let multiplier = options.sources.len();
    let (tx, mut rx) = tokio::sync::mpsc::channel(100 * multiplier);

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
                timesource: tmsg.timesource,
                rssi: tmsg.rssi,
                frame: tmsg.frame.to_string(),
                message: Some(msg),
                idx: tmsg.idx,
            };
            let mut reference =
                app_dec.lock().await.sources[tmsg.idx].reference;

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

            // References may have been modified.
            // With static receivers, we don't care; for dynamic ones, we may
            // want to update the reference position.
            if options.update_position {
                app_dec.lock().await.sources[tmsg.idx].reference = reference;
            }

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
    sources: Vec<Source>,
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
    ALTITUDE,
    VRATE,
    #[default]
    COUNT,
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
                Char('.') => {
                    jet1090.sort_key = SortKey::COUNT;
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
    pub fn receivers(&mut self) {
        for source in &mut self.sources {
            source.count = 0;
        }
        for vector in self.state_vectors.values_mut() {
            self.sources[vector.cur.idx]
                .airport
                .clone_into(&mut vector.cur.airport);
            self.sources[vector.cur.idx].count += 1;
            if self.sources[vector.cur.idx].last < vector.cur.last {
                self.sources[vector.cur.idx].last = vector.cur.last
            }
        }
    }
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

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[cfg(test)]
mod tests {

    use crate::Options;

    #[test]
    fn test_config() {
        let options: Options = toml::from_str(
            r#"
            verbose = false
            interactive = true
            serve_port = 8080
            expire = 1
            update_position = false

            [[sources]]
            host = '0.0.0.0'
            port = 1234
            rtlsdr = false
            airport = 'LFBO'

            [[sources]]
            host = '0.0.0.0'
            port = 3456
            rtlsdr = false
            [reference]
            latitude = 48.723
            longitude = 2.379
            "#,
        )
        .unwrap();

        assert!(options.interactive);
        assert_eq!(options.sources.len(), 2);
    }
}
