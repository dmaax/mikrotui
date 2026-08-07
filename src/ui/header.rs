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

    // Left Title Block
    let title_spans = vec![
        Span::styled(" MikroTUI ", t.title),
        Span::styled(" v0.1.0 ", t.muted_text),
        Span::styled(" (WinBox TUI) ", t.accent.add_modifier(Modifier::BOLD)),
    ];
    let title_p = Paragraph::new(Line::from(title_spans))
        .block(Block::default().borders(Borders::ALL).border_style(t.border));
    f.render_widget(title_p, chunks[0]);

    // Right Status Block
    let host_info = format!(" Router: {} ({}) ", app.client.config.host, app.system_resource.board_name);

    let (safe_badge, safe_style) = if app.safe_mode {
        (" [SAFE MODE: ENABLED] ", t.success.add_modifier(Modifier::BOLD))
    } else {
        (" [SAFE MODE: DISABLED] ", t.danger.add_modifier(Modifier::BOLD))
    };

    let status_spans = vec![
        Span::styled(host_info, t.normal_text),
        Span::raw(" | "),
        Span::styled(safe_badge, safe_style),
        Span::raw(" | "),
        Span::styled(" [READ-ONLY] ", t.warning.add_modifier(Modifier::BOLD)),
    ];

    let status_p = Paragraph::new(Line::from(status_spans))
        .block(Block::default().borders(Borders::ALL).border_style(t.border));
    f.render_widget(status_p, chunks[1]);
}
