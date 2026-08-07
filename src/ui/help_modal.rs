use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};
use crate::app::App;

pub fn render_help_modal(f: &mut Frame, app: &App) {
    if !app.show_help_modal {
        return;
    }

    let t = &app.theme;

    let area = centered_rect(65, 70, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_focus)
        .title(Span::styled(" ⌨  MikroTUI Keyboard Shortcuts & Help ", t.title));

    let header_cells = ["Hotkey / Shortcut", "Description & Action"]
        .iter()
        .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let shortcuts = vec![
        ("Tab / Shift+Tab", "Switch active menu tab (or use Left/Right Arrow keys / h / l)"),
        ("↑ / ↓ (or k / j)", "Navigate up / down through table rows"),
        ("Enter", "Open Item Details modal (view full properties & complete comments)"),
        ("p", "Open interactive Ping Diagnostic tool (/ping <target>)"),
        ("/", "Activate live filter search mode (type query, Enter/Esc to finish)"),
        ("t", "Cycle color themes (WinBox Dark, Nord Slate, High Contrast)"),
        ("Ctrl+X", "Toggle Safe Mode indicator (Safe mode is enabled by default)"),
        ("r / F5", "Refresh all data via SSH in background (non-blocking)"),
        ("?", "Toggle this Keyboard Shortcuts & Help modal window"),
        ("q / Ctrl+C", "Quit MikroTUI"),
    ];

    let rows = shortcuts.into_iter().map(|(key, desc)| {
        Row::new(vec![
            Cell::from(key).style(t.accent.add_modifier(Modifier::BOLD)),
            Cell::from(desc).style(t.normal_text),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Min(35),
        ],
    )
    .header(header)
    .block(Block::default());

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Shortcuts table
            Constraint::Length(2), // Footer instruction
        ])
        .split(inner_area);

    f.render_widget(block, area);
    f.render_widget(table, inner_chunks[0]);

    let footer_p = Paragraph::new(Line::from(Span::styled(
        " (Press Enter, Esc, or '?' to close this help window) ",
        t.muted_text,
    )))
    .alignment(Alignment::Center);

    f.render_widget(footer_p, inner_chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
