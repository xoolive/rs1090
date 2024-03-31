use chrono::prelude::*;
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::time::{SystemTime, UNIX_EPOCH};
use style::palette::tailwind;

use crate::{Jet1090, SortKey};

const INFO_TEXT: &str = "(Esc/Q) quit | (↑/K) up | (↓/J) down ";

fn ts_to_utc(timestamp: u64) -> String {
    let dt: DateTime<Utc> =
        DateTime::from_timestamp(timestamp as i64, 0).unwrap();
    format!("{}", dt.format("%H:%M"))
}

pub fn build_table(frame: &mut Frame, app: &mut Jet1090) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before unix epoch")
        .as_secs();

    let states = &app.state_vectors;

    app.items = states
        .values()
        .filter(|sv| (now as i64 - sv.cur.last as i64) < 30)
        .map(|sv| sv.cur.icao24.to_string())
        .collect();

    app.scroll_state = app.scroll_state.content_length(app.items.len());

    let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(1)])
        .split(frame.size());
    let colors = TableColors::new(&tailwind::CYAN);

    use crate::snapshot::StateVectors;
    let mut sorted_elts = states.values().collect::<Vec<&StateVectors>>();

    let sort_by = match &app.sort_key {
        SortKey::ALTITUDE => |a: &&StateVectors, b: &&StateVectors| {
            a.cur.altitude.cmp(&b.cur.altitude)
        },
        SortKey::CALLSIGN => |a: &&StateVectors, b: &&StateVectors| {
            a.cur.callsign.cmp(&b.cur.callsign)
        },
        SortKey::VRATE => |a: &&StateVectors, b: &&StateVectors| {
            a.cur.vertical_rate.cmp(&b.cur.vertical_rate)
        },
        SortKey::FIRST => {
            |a: &&StateVectors, b: &&StateVectors| a.cur.first.cmp(&b.cur.first)
        }
        SortKey::LAST => {
            |a: &&StateVectors, b: &&StateVectors| a.cur.last.cmp(&b.cur.last)
        }
    };

    sorted_elts.sort_by(sort_by);
    if !&app.sort_asc {
        sorted_elts.reverse();
    }

    let rows = sorted_elts
        .iter()
        .filter(|sv| (now as i64 - sv.cur.last as i64) < 30)
        .enumerate()
        .map(|(i, sv)| {
            let color = match i % 2 {
                0 => colors.normal_row_color,
                _ => colors.alt_row_color,
            };
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
                match (sv.cur.selected_altitude, sv.cur.altitude) {
                    (Some(sel), Some(alt)) if u16::abs_diff(sel, alt) <= 50 => {
                        "=".to_string()
                    }
                    (Some(sel), _) => {
                        format!("{}", sel / 100)
                    }
                    _ => "".to_string(),
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
            .style(Style::new().fg(colors.row_fg).bg(color))
        })
        .collect::<Vec<Row<'_>>>();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(4),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(5),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(7),
        Constraint::Length(5),
    ];

    let size = &rows.len();
    let bar = "█";
    let mut callsign_style = Style::default();
    let mut altitude_style = Style::default();
    let mut vrate_style = Style::default();
    let mut first_style = Style::default();
    let mut last_style = Style::default();
    let hl = tailwind::AMBER.c400;
    match app.sort_key {
        SortKey::CALLSIGN => {
            callsign_style = callsign_style.fg(hl);
        }
        SortKey::ALTITUDE => {
            altitude_style = altitude_style.fg(hl);
        }
        SortKey::VRATE => {
            vrate_style = vrate_style.fg(hl);
        }
        SortKey::FIRST => {
            first_style = first_style.fg(hl);
        }
        SortKey::LAST => {
            last_style = last_style.fg(hl);
        }
    }
    let colnames = vec![
        Cell::from("icao24"),
        Cell::from("callsign").style(callsign_style),
        Cell::from("sqwk"),
        Cell::from("lat"),
        Cell::from("lon"),
        Cell::from("alt").style(altitude_style),
        Cell::from("sel"),
        Cell::from("gs"),
        Cell::from("tas"),
        Cell::from("ias"),
        Cell::from("mach"),
        Cell::from("vrate").style(vrate_style),
        Cell::from("trk"),
        Cell::from("hdg"),
        Cell::from("roll"),
        Cell::from("nacp"),
        Cell::from("last").style(last_style),
        Cell::from("first").style(first_style),
    ];
    let table = Table::new(rows, widths)
        .column_spacing(2)
        .header(
            Row::new(colnames)
                //.bottom_margin(1)
                .style(
                    Style::default()
                        .fg(colors.header_fg)
                        .bg(colors.header_bg)
                        .bold(),
                ),
        )
        .block(
            Block::default()
                .title_bottom(format!("jet1090 ({} aircraft)", size,))
                .title_alignment(Alignment::Right)
                .title_style(Style::new().blue().bold())
                .padding(Padding::symmetric(1, 0))
                .borders(Borders::ALL),
        )
        .bg(colors.buffer_bg)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(colors.selected_style_fg),
        )
        .highlight_symbol(bar)
        .highlight_spacing(HighlightSpacing::Always);

    let area = rects[0];
    frame.render_stateful_widget(table, area, &mut app.state);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        area.inner(&Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.scroll_state,
    );

    let area = rects[1];
    frame.render_widget(
        Paragraph::new(Line::from(INFO_TEXT))
            .style(Style::new().fg(colors.row_fg).bg(colors.buffer_bg))
            .centered(),
        area,
    );
}

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    //footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            //footer_border_color: color.c400,
        }
    }
}
