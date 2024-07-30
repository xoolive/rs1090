#![doc = include_str!("../readme.md")]
#![allow(unused)]

mod aircraftdb;
mod cli;
mod snapshot;
mod table;
mod tui;
mod web;

use clap::{Command, CommandFactory, Parser, ValueHint};
use clap_complete::{generate, Generator, Shell};
mod channel;
mod websocket;

use crate::channel::{ChannelControl, ChannelMessage};
use crate::websocket::{
    jet1090_data_task, on_connected, system_datetime_task, State,
};

use clap::Parser;
use cli::Source;
use crossterm::event::KeyCode;
use ratatui::widgets::*;
use rs1090::decode::cpr::{decode_position, AircraftState};
use rs1090::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::connect_async;
use tracing::{debug, error, info};
use tui::Event;
use warp::Filter;
use web::TrackQuery;

#[derive(Default, Deserialize, Parser)]
use futures::SinkExt;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Error};

use uuid::Uuid;
use warp::ws::WebSocket;

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

    /// Use websocket data source
    /// This is workaround to use binary data source from another websocket server
    /// To use this, you will also need a source `127.0.0.1:42125@LFBO`
    #[arg(long, default_value = "false")]
    use_websocket_source: bool,

    #[arg(long, default_value = "127.0.0.1")]
    websocket_host: String,

    /// Port for the websocket serve_port
    /// Specifying the port also imply that the websocket server is enabled
    #[arg(long, default_value = None)]
    websocket_port: Option<u16>,

    #[arg(long, default_value = "false")]
    websocket: bool,
}

/// this subscribe to binary data from a websocket and send it to a local udp server
/// we could just implement this int `Source::receiver`
async fn websocket_client() {
    let websocket_url = "ws://51.158.72.24:1234/42125@LFBO";
    let local_udp = "127.0.0.1:42125"; // you have to specify a source like 127.0.0.1:42125@LFBO

    loop {
        // create a UDP client socket
        // put all things in a loop to retry in case of failure
        let udp_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        match udp_socket.connect(local_udp).await {
            Ok(_) => {}
            Err(err) => {
                info!(
                    "failed to connect to udp://{}, {:?}, retry in 1 second(s)",
                    local_udp, err
                );
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        }

        // connect to the websocket data source
        let (websocket_stream, _) = connect_async(websocket_url)
            .await
            .expect("fail to connect to websocket endpoint");
        info!("connected to {}", websocket_url);

        // just receive data from the websocket and send it to the udp server
        let (_, websocket_rx) = websocket_stream.split();
        websocket_rx
            .for_each(|message| async {
                let raw_bytes = message.unwrap().into_data();
                // code: 111, kind: ConnectionRefused
                let result = udp_socket.send(&raw_bytes).await;
                if result.is_err() {
                    error!("fail to send, {}, {:?}", local_udp, result);
                }
                // debug!("raw data sent, size: {:?}", raw_bytes.len());
            })
            .await;
    }
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
    tracing_subscriber::fmt()
        // .with_env_filter("jet1090=info,jet1090::websocket=debug")
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

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

    // rs1090 data items (TimeedMessage) are sent to timed_message_tx
    // a thread reads from timed_message_stream and relay them to channels
    let (timed_message_tx, timed_message_rx) =
        tokio::sync::mpsc::unbounded_channel();
    let timed_message_stream = UnboundedReceiverStream::new(timed_message_rx);

    let channel_control = ChannelControl::new();
    channel_control.new_channel("phoenix".into(), None).await; // channel for server to publish heartbeat
    channel_control.new_channel("system".into(), None).await;
    channel_control.new_channel("jet1090".into(), None).await;

    let state = Arc::new(State {
        ctl: Mutex::new(channel_control),
    });
    if options.serve_port.is_some() {
        tokio::spawn(system_datetime_task(state.clone(), "system"));
        tokio::spawn(jet1090_data_task(
            state.clone(),
            timed_message_stream,
            "jet1090",
            "data",
        ));
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
                .allow_methods(vec!["GET", "POST"]);

            let ws_state = state.clone();
            let ws_route = warp::path("websocket")
                .and(warp::ws())
                .and(warp::any().map(move || ws_state.clone()))
                .map(|ws: warp::ws::Ws, state| {
                    ws.on_upgrade(move |websocket| {
                        on_connected(websocket, state)
                    })
                });

            let channels_assets = warp::path("channels")
                .and(warp::fs::dir("./crates/jet1090/src/assets"));

            #[derive(Debug, Clone, Serialize, Deserialize)]
            struct Code {
                agent_id: String,
                channel_name: String,
                code: String,
            }
            let channel_control_state = state.clone();
            let update_filter_route = warp::path!("channels" / "filters")
                .and(warp::post())
                .and(warp::body::json())
                .and(warp::any().map(move || channel_control_state.clone()))
                .and_then(|code: Code, state: Arc<State>| async move {
                    state
                        .ctl
                        .lock()
                        .await
                        .channel_map
                        .lock()
                        .await
                        .get(code.channel_name.as_str())
                        .unwrap()
                        .send(ChannelMessage::ReloadFilter {
                            agent_id: code.agent_id,
                            code: code.code,
                        })
                        .unwrap();
                    Ok::<_, std::convert::Infallible>(warp::reply::json(&0))
                });

            let routes = warp::get()
                .and(
                    home.or(all)
                        .or(track)
                        .or(receivers)
                        .or(channels_assets)
                        .or(ws_route),
                )
                .or(update_filter_route)
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

    if options.use_websocket_source {
        tokio::spawn(websocket_client());
    }

    while let Some(tmsg) = rx.recv().await {
        let frame = hex::decode(&tmsg.frame).unwrap();
        if let Ok((_, msg)) = Message::from_bytes((&frame, 0)) {
            let mut msg = TimedMessage {
                timestamp: tmsg.timestamp,
                timesource: tmsg.timesource,
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

            if options.serve_port.is_some() {
                // http server is enabled
                // send the message to channel `jet1090` event `data`
                // on the other side, thread `rs1090_data_task` receives and publishes the message to all clients
                timed_message_tx.send(msg.clone())?;
            }

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
