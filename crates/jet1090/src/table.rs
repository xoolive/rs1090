use chrono::prelude::*;
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::time::{SystemTime, UNIX_EPOCH};
use style::palette::tailwind;

use crate::snapshot::Snapshot;
use crate::{Jet1090, SortKey};

const INFO_TEXT: &str = "(Esc/Q) quit | (↑/K) up | (↓/J) down | (⤒/G) top";

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
    let columns = {
        use ColumnRender::*;
        match app.width {
            w if w <= 70 => {
                vec![
                    ICAO24,
                    CALLSIGN,
                    LATITUDE,
                    LONGITUDE,
                    ALTITUDE,
                    GROUNDSPEED,
                    TRACK,
                ]
            }
            w if w <= 80 => {
                vec![
                    ICAO24,
                    CALLSIGN,
                    LATITUDE,
                    LONGITUDE,
                    ALTITUDE,
                    GROUNDSPEED,
                    TRACK,
                    LAST,
                ]
            }
            w if w <= 100 => {
                vec![
                    ICAO24,
                    CALLSIGN,
                    SQUAWK,
                    LATITUDE,
                    LONGITUDE,
                    ALTITUDE,
                    GROUNDSPEED,
                    VRATE,
                    TRACK,
                    LAST,
                    FIRST,
                ]
            }
            w if w <= 120 => {
                vec![
                    ICAO24,
                    CALLSIGN,
                    SQUAWK,
                    LATITUDE,
                    LONGITUDE,
                    ALTITUDE,
                    GROUNDSPEED,
                    VRATE,
                    TRACK,
                    NACP,
                    LAST,
                    FIRST,
                ]
            }
            w if w <= 130 => {
                vec![
                    ICAO24,
                    CALLSIGN,
                    SQUAWK,
                    LATITUDE,
                    LONGITUDE,
                    ALTITUDE,
                    SELALT,
                    GROUNDSPEED,
                    TAS,
                    IAS,
                    MACH,
                    VRATE,
                    TRACK,
                    HEADING,
                    ROLL,
                    NACP,
                    LAST,
                    FIRST,
                ]
            }
            _ => {
                vec![
                    ICAO24,
                    TAIL,
                    CALLSIGN,
                    SQUAWK,
                    LATITUDE,
                    LONGITUDE,
                    ALTITUDE,
                    SELALT,
                    GROUNDSPEED,
                    TAS,
                    IAS,
                    MACH,
                    VRATE,
                    TRACK,
                    HEADING,
                    ROLL,
                    NACP,
                    LAST,
                    FIRST,
                ]
            }
        }
    };
    let rows = sorted_elts
        .iter()
        .filter(|sv| (now as i64 - sv.cur.last as i64) < 30)
        .enumerate()
        .map(|(i, sv)| {
            let color = match i % 2 {
                0 => colors.normal_row_color,
                _ => colors.alt_row_color,
            };
            columns
                .iter()
                .map(|c| c.cell(&sv.cur, now))
                .collect::<Row<'_>>()
                .style(Style::new().fg(colors.row_fg).bg(color))
        })
        .collect::<Vec<Row<'_>>>();

    let size = &rows.len();
    let bar = "█";

    let header = columns
        .iter()
        .map(|c| c.header(&app.sort_key))
        .collect::<Vec<Cell<'_>>>();

    let constraints = columns
        .iter()
        .map(|c| c.constraint())
        .collect::<Vec<Constraint>>();

    let table = Table::new(rows, constraints)
        .column_spacing(2)
        .header(
            Row::new(header)
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

trait Render {
    fn cell(&self, snapshot: &Snapshot, now: u64) -> String;
    fn header(&self, sort_key: &SortKey) -> Cell;
    fn constraint(&self) -> Constraint;
}

#[allow(clippy::upper_case_acronyms)]
enum ColumnRender {
    ICAO24,
    TAIL,
    CALLSIGN,
    SQUAWK,
    LATITUDE,
    LONGITUDE,
    ALTITUDE,
    SELALT,
    GROUNDSPEED,
    TAS,
    IAS,
    MACH,
    VRATE,
    TRACK,
    HEADING,
    ROLL,
    NACP,
    LAST,
    FIRST,
}

impl Render for ColumnRender {
    fn cell(&self, s: &Snapshot, now: u64) -> String {
        match self {
            Self::ICAO24 => s.icao24.to_string(),
            Self::TAIL => {
                let hexid = u32::from_str_radix(&s.icao24, 16).unwrap_or(0);
                rs1090::data::tail::tail(hexid).unwrap_or("".to_string())
            }
            Self::CALLSIGN => s.callsign.to_owned().unwrap_or("".to_string()),
            Self::SQUAWK => {
                s.squawk.map(|s| s.to_string()).unwrap_or("".to_string())
            }
            Self::LATITUDE => s
                .latitude
                .map(|v| format!("{}", v))
                .unwrap_or("".to_string()),
            Self::LONGITUDE => s
                .longitude
                .map(|v| format!("{}", v))
                .unwrap_or("".to_string()),
            Self::ALTITUDE => s
                .altitude
                .map(|v| format!("{}", v))
                .unwrap_or("".to_string()),
            Self::SELALT => match (s.selected_altitude, s.altitude) {
                (Some(sel), Some(alt)) if u16::abs_diff(sel, alt) <= 50 => {
                    "=".to_string()
                }
                (Some(sel), _) => {
                    format!("{}", sel / 100)
                }
                _ => "".to_string(),
            },
            Self::GROUNDSPEED => s
                .groundspeed
                .map(|v| format!("{}", v))
                .unwrap_or("".to_string()),
            Self::TAS => {
                s.tas.map(|v| format!("{}", v)).unwrap_or("".to_string())
            }
            Self::IAS => {
                s.ias.map(|v| format!("{}", v)).unwrap_or("".to_string())
            }
            Self::MACH => {
                s.mach.map(|v| format!("{}", v)).unwrap_or("".to_string())
            }
            Self::VRATE => s
                .vertical_rate
                .map(|v| format!("{}", v))
                .unwrap_or("".to_string()),
            Self::TRACK => {
                s.track.map(|v| format!("{}", v)).unwrap_or("".to_string())
            }
            Self::HEADING => s
                .heading
                .map(|v| format!("{}", v))
                .unwrap_or("".to_string()),
            Self::ROLL => {
                s.roll.map(|v| format!("{}", v)).unwrap_or("".to_string())
            }
            Self::NACP => {
                s.nacp.map(|v| format!("{}", v)).unwrap_or("".to_string())
            }
            Self::LAST => {
                if now > s.last + 5 {
                    format!("{}s ago", now - s.last)
                } else {
                    "".to_string()
                }
            }
            Self::FIRST => {
                let dt: DateTime<Utc> =
                    DateTime::from_timestamp(s.first as i64, 0).unwrap();
                format!("{}", dt.format("%H:%M"))
            }
        }
    }

    fn header(&self, sort_key: &SortKey) -> Cell {
        match self {
            ColumnRender::ICAO24 => Cell::from("icao24".to_string()),
            ColumnRender::TAIL => Cell::from("tail".to_string()),
            ColumnRender::CALLSIGN => {
                let mut c = Cell::from("callsign".to_string());
                if *sort_key == SortKey::CALLSIGN {
                    c = c.fg(tailwind::AMBER.c400);
                }
                c
            }
            ColumnRender::SQUAWK => Cell::from("sqwk".to_string()),
            ColumnRender::LATITUDE => Cell::from("lat".to_string()),
            ColumnRender::LONGITUDE => Cell::from("lon".to_string()),
            ColumnRender::ALTITUDE => {
                let mut c = Cell::from("alt".to_string());
                if *sort_key == SortKey::ALTITUDE {
                    c = c.fg(tailwind::AMBER.c400);
                }
                c
            }
            ColumnRender::SELALT => Cell::from("sel".to_string()),
            ColumnRender::GROUNDSPEED => Cell::from("gs".to_string()),
            ColumnRender::TAS => Cell::from("tas".to_string()),
            ColumnRender::IAS => Cell::from("ias".to_string()),
            ColumnRender::MACH => Cell::from("mach".to_string()),
            ColumnRender::VRATE => {
                let mut c = Cell::from("vrate".to_string());
                if *sort_key == SortKey::VRATE {
                    c = c.fg(tailwind::AMBER.c400);
                }
                c
            }
            ColumnRender::TRACK => Cell::from("trk".to_string()),
            ColumnRender::HEADING => Cell::from("hdg".to_string()),
            ColumnRender::ROLL => Cell::from("roll".to_string()),
            ColumnRender::NACP => Cell::from("nac".to_string()),
            ColumnRender::LAST => {
                let mut c = Cell::from("last".to_string());
                if *sort_key == SortKey::LAST {
                    c = c.fg(tailwind::AMBER.c400);
                }
                c
            }
            ColumnRender::FIRST => {
                let mut c = Cell::from("first".to_string());
                if *sort_key == SortKey::FIRST {
                    c = c.fg(tailwind::AMBER.c400);
                }
                c
            }
        }
    }
    fn constraint(&self) -> Constraint {
        match self {
            ColumnRender::ICAO24 => Constraint::Length(6),
            ColumnRender::TAIL => Constraint::Length(8),
            ColumnRender::CALLSIGN => Constraint::Length(8),
            ColumnRender::SQUAWK => Constraint::Length(4),
            ColumnRender::LATITUDE => Constraint::Length(6),
            ColumnRender::LONGITUDE => Constraint::Length(6),
            ColumnRender::ALTITUDE => Constraint::Length(5),
            ColumnRender::SELALT => Constraint::Length(3),
            ColumnRender::GROUNDSPEED => Constraint::Length(3),
            ColumnRender::TAS => Constraint::Length(3),
            ColumnRender::IAS => Constraint::Length(3),
            ColumnRender::MACH => Constraint::Length(4),
            ColumnRender::VRATE => Constraint::Length(5),
            ColumnRender::TRACK => Constraint::Length(5),
            ColumnRender::HEADING => Constraint::Length(5),
            ColumnRender::ROLL => Constraint::Length(5),
            ColumnRender::NACP => Constraint::Length(3),
            ColumnRender::LAST => Constraint::Length(7),
            ColumnRender::FIRST => Constraint::Length(5),
        }
    }
}
