use chrono::prelude::*;
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::snapshot::StateVectors;

fn ts_to_utc(timestamp: u64) -> String {
    let dt: DateTime<Utc> =
        DateTime::from_timestamp(timestamp as i64, 0).unwrap();
    format!("{}", dt.format("%H:%M:%S"))
}

pub async fn build_rows(
    states_tui: &BTreeMap<String, StateVectors>,
) -> Vec<Row> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before unix epoch")
        .as_secs();

    let rows: Vec<Row> = states_tui
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
                if let Some(nac) = sv.cur.nacp {
                    format!("{}", nac)
                } else {
                    "".to_string()
                },
                if now > sv.cur.last + 5 {
                    format!("{}s ago", now - sv.cur.last)
                } else {
                    "".to_string()
                },
                ts_to_utc(sv.cur.first),
            ])
        })
        .collect();
    rows
}

pub fn build_table(frame: &mut Frame, rows: Vec<Row>, vertical_scroll: usize) {
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
                "nacp", "last", "first",
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
        .highlight_symbol(">>")
        .highlight_spacing(HighlightSpacing::Always);

    let area = frame.size();

    frame.render_widget(table, area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state =
        ScrollbarState::new(*size).position(vertical_scroll);
    frame.render_stateful_widget(
        scrollbar,
        area.inner(&Margin {
            // using an inner vertical margin of 1 unit makes the scrollbar inside the block
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}
