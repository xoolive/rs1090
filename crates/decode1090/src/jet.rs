#![doc = include_str!("../readme.md")]

mod snapshot;
mod table;
mod tui;

use clap::Parser;
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use rs1090::decode::cpr::{decode_position, AircraftState, Position};
use rs1090::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
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

    let app_tui = Arc::new(Mutex::new(Jet1090 {
        scroll_pos: 0,
        should_quit: false,
        state_vectors: BTreeMap::new(),
    }));
    let app_dec = app_tui.clone();

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

    tokio::spawn(async move {
        loop {
            if let Ok(event) = events.next().await {
                let _ = update(&mut app_tui.lock().await, event);
            } // new
            let app = app_tui.lock().await;
            if app.should_quit {
                break;
            }
            let states = &app.state_vectors;
            let rows = table::build_rows(states).await;
            let counter = app.scroll_pos;
            terminal.draw(|frame| table::build_table(frame, rows, counter))?;
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

            snapshot::update_snapshot(&app_dec, &mut msg).await;

            let json = serde_json::to_string(&msg).unwrap();
            if options.verbose {
                println!("{}", json);
            }
            if let Some(file) = &mut file {
                file.write_all(json.as_bytes()).await?;
                file.write_all("\n".as_bytes()).await?;
            }
        }
        if app_dec.lock().await.should_quit {
            break;
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct Jet1090 {
    scroll_pos: usize,
    should_quit: bool,
    state_vectors: BTreeMap<String, snapshot::StateVectors>,
}

fn update(
    jet1090: &mut tokio::sync::MutexGuard<Jet1090>,
    event: Event,
) -> Result<()> {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('j') => jet1090.scroll_pos += 1,
            KeyCode::Char('k') => {
                jet1090.scroll_pos = if jet1090.scroll_pos == 0 {
                    0
                } else {
                    jet1090.scroll_pos - 1
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => jet1090.should_quit = true,
            _ => {}
        }
    }
    Ok(())
}
