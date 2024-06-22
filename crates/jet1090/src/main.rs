#![doc = include_str!("../readme.md")]
#![allow(unused)]

mod aircraftdb;
mod cli;
mod snapshot;
mod table;
mod tui;
mod web;

mod channels;
mod websocket;

use crate::channels::ChannelControl;
use crate::websocket::{
    handle_incoming_messages, rs1090_data_task, timestamp_task, State, User,
};

use clap::Parser;
use cli::Source;
use crossterm::event::KeyCode;
use log::{debug, info};
use ratatui::widgets::*;
use rs1090::decode::cpr::{decode_position, AircraftState};
use rs1090::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::connect_async;
use tui::Event;
use warp::Filter;
use web::TrackQuery;

use futures::SinkExt;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};
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

    /// Should we update the reference positions (if the receiver is moving)
    #[arg(short, long, default_value = "false")]
    update_position: bool,

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
                udp_socket.send(&raw_bytes).await.unwrap();
                // debug!("raw data sent, size: {:?}", raw_bytes.len());
            })
            .await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

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
        channels: Mutex::new(channel_control),
    });
    if options.serve_port.is_some() {
        tokio::spawn(timestamp_task(state.clone(), "system"));
        tokio::spawn(rs1090_data_task(
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
                .allow_methods(vec!["GET"]);

            let state = warp::any().map(move || state.clone());
            let ws_route = warp::path("websocket")
                .and(warp::ws())
                .and(state)
                .map(|ws: warp::ws::Ws, state| {
                    ws.on_upgrade(move |websocket| {
                        on_connected(websocket, state)
                    })
                });

            let channels_assets = warp::path("channels")
                .and(warp::fs::dir("./crates/jet1090/src/assets"));

            let routes = warp::get()
                .and(
                    home.or(all)
                        .or(track)
                        .or(receivers)
                        .or(channels_assets)
                        .or(ws_route),
                )
                .recover(web::handle_rejection)
                .with(cors);

            warp::serve(routes).run(([0, 0, 0, 0], port)).await;
        });
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

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

pub async fn on_connected(ws: WebSocket, state: Arc<State>) {
    let (mut tx, mut rx) = ws.split();

    let user = User::default();
    info!("user: {} connected", user.user_id);

    state
        .channels
        .lock()
        .await
        .add_user(user.user_id.to_string(), None)
        .await;

    // get receiver for user that get message from all channels
    let mut user_receiver = state
        .channels
        .lock()
        .await
        .get_user_receiver(user.user_id.to_string())
        .await
        .unwrap();

    // channels => websocket client
    let mut tx_task = tokio::spawn(async move {
        while let Ok(my_message) = user_receiver.recv().await {
            tx.send(warp::ws::Message::text(my_message)).await.unwrap();
        }
    });

    // spawn a task to get message from user and handle things
    let rec_state = state.clone();
    let mut rx_task = tokio::spawn(async move {
        while let Some(result) = rx.next().await {
            if let Ok(message) = result {
                if message.is_text() {
                    handle_incoming_messages(
                        message.to_str().unwrap().to_string(),
                        rec_state.clone(),
                        &user.user_id.clone(),
                    )
                    .await;
                }
            };
        }
    });

    tokio::select! {
        _ = (&mut tx_task) => rx_task.abort(),
        _ = (&mut rx_task) => tx_task.abort(),
    }

    state
        .channels
        .lock()
        .await
        .remove_user(user.session_id.to_string())
        .await;
    info!("client connection closed");
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
