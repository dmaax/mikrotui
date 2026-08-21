use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(35), // Title & App Name
            Constraint::Min(30),    // Active Host & Safe Mode Status
        ])
        .split(area);

    // Left Title Block (version dynamically extracted from Cargo.toml at compile time)
    let version_str = format!(" v{} ", env!("CARGO_PKG_VERSION"));
    let title_spans = vec![
        Span::styled(" MikroTUI ", t.title),
        Span::styled(version_str, t.muted_text),
        Span::styled(" (WinBox TUI) ", t.accent.add_modifier(Modifier::BOLD)),
    ];
    let title_p = Paragraph::new(Line::from(title_spans))
        .block(Block::default().borders(Borders::ALL).border_style(t.border));
    f.render_widget(title_p, chunks[0]);

    // Right Status Block
    let host_info = format!(" Router: {} ({}) ", app.client.config.host, app.system_resource.board_name);

    // Reports the actual state of host key verification rather than a badge that was
    // decorative: in demo mode there is no SSH session to verify at all.
    let (key_badge, key_style) = if app.client.config.demo_mode {
        (" [DEMO] ", t.host_key_unverified)
    } else if app.host_key_verified {
        (" [HOST KEY: VERIFIED] ", t.host_key_verified)
    } else {
        (" [HOST KEY: UNVERIFIED] ", t.host_key_unverified)
    };

    let status_spans = vec![
        Span::styled(host_info, t.normal_text),
        Span::raw(" | "),
        Span::styled(key_badge, key_style),
        Span::raw(" | "),
        Span::styled(" [READ-ONLY] ", t.read_only_badge),
    ];

    let status_p = Paragraph::new(Line::from(status_spans))
        .block(Block::default().borders(Borders::ALL).border_style(t.border));
    f.render_widget(status_p, chunks[1]);
}
