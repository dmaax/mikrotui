use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// One row, no border.
///
/// This was a three-row box holding a single line of text — an eighth of a 24-row
/// terminal spent on chrome. With the status bar the same, four rows go back to the
/// table, a third more data on a short screen.
pub const HEIGHT: u16 = 1;

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    // The badges report security state and are the reason to look at this row at all, so
    // they claim their width first. They used to sit on the right of a fixed split, which
    // made `[READ-ONLY]` the first thing an 80-column terminal cut off.
    let (key_badge, key_style) = if app.client.config.demo_mode {
        (" DEMO ", t.host_key_unverified)
    } else if app.host_key_verified {
        (" HOST KEY: VERIFIED ", t.host_key_verified)
    } else {
        (" HOST KEY: UNVERIFIED ", t.host_key_unverified)
    };
    let read_only = " READ-ONLY ";
    let badge_width = (key_badge.len() + read_only.len() + 1) as u16;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(8), Constraint::Length(badge_width)])
        .split(area);

    // Identity takes what is left, in the longest form that fits.
    let host = &app.client.config.host;
    let board = &app.system_resource.board_name;
    let version = env!("CARGO_PKG_VERSION");

    let with_board = if board.is_empty() {
        format!(" {host} ")
    } else {
        format!(" {host} ({board}) ")
    };
    let full = format!(" MikroTUI {version} · {}", with_board.trim_start());

    let room = chunks[0].width as usize;
    let identity = if full.chars().count() <= room {
        full
    } else if with_board.chars().count() <= room {
        with_board
    } else {
        format!(" {host} ")
    };

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            identity,
            t.title.add_modifier(Modifier::BOLD),
        ))),
        chunks[0],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(key_badge, key_style),
            Span::raw(" "),
            Span::styled(read_only, t.read_only_badge),
        ]))
        .alignment(Alignment::Right),
        chunks[1],
    );
}
