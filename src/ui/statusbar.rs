use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::{App, InputMode};

pub fn render_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(40),   // Help hotkeys & status
            Constraint::Length(35), // Filter input bar
        ])
        .split(area);

    // Left status message & hotkeys
    let status_spans = vec![
        Span::styled(" [Tab] Switch ", t.accent),
        Span::styled(" [Enter] Details ", t.title),
        Span::styled(" [p] Ping ", t.success),
        Span::styled(" [Ctrl+O] Switch Host ", t.title),
        Span::styled(" [Ctrl+X] Safe Mode ", t.warning),
        Span::styled(" [/] Search ", t.warning),
        Span::styled(" [t] Theme ", t.accent),
        Span::styled(" [?] Help ", t.title),
        Span::styled(" [r] Reload ", t.accent),
        Span::styled(" [q] Quit ", t.danger),
        Span::raw(" | "),
        Span::styled(&app.status_message, t.muted_text),
    ];

    let status_p = Paragraph::new(Line::from(status_spans))
        .block(Block::default().borders(Borders::ALL).border_style(t.border));
    f.render_widget(status_p, chunks[0]);

    // Right filter bar
    let (filter_text, filter_style) = match app.input_mode {
        InputMode::Filtering => (
            format!(" Search: {}█ ", app.filter_query),
            t.selected_row,
        ),
        InputMode::Normal => {
            if app.filter_query.is_empty() {
                (" Search: (press '/' to filter)".to_string(), t.muted_text)
            } else {
                (format!(" Active Filter: {} ", app.filter_query), t.warning)
            }
        }
    };

    let filter_p = Paragraph::new(Line::from(Span::styled(filter_text, filter_style)))
        .block(Block::default().borders(Borders::ALL).border_style(t.border).title("Filter"));
    f.render_widget(filter_p, chunks[1]);
}
