use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};
use crate::app::App;

pub fn render_host_switch_modal(f: &mut Frame, app: &App) {
    if !app.show_host_switch_modal {
        return;
    }

    let t = &app.theme;

    let area = centered_rect(65, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_focus)
        .title(Span::styled(" 🔀 Quick Host Switcher (Ctrl+O) ", t.title));

    if app.available_hosts.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(" ⚠️ No stored hosts found in ~/.config/mikrotui/config.json ", t.warning)),
            Line::from(""),
            Line::from(Span::styled(" Use 'mikrotui host add' to register new router hosts. ", t.muted_text)),
            Line::from(""),
            Line::from(Span::styled(" (Press Esc or Ctrl+O to close) ", t.muted_text)),
        ])
        .block(block)
        .alignment(Alignment::Center);

        f.render_widget(p, area);
        return;
    }

    let header_cells = ["#", "Router Alias / Name", "IP Address / Host", "Port", "User"]
        .iter()
        .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.available_hosts.iter().enumerate().map(|(idx, h)| {
        let is_selected = idx == app.host_switch_selected;
        let row_style = if is_selected { t.selected_row } else { t.normal_text };

        Row::new(vec![
            Cell::from(format!("{}", idx + 1)),
            Cell::from(h.name.as_str()).style(t.accent.add_modifier(Modifier::BOLD)),
            Cell::from(h.host.as_str()).style(t.success),
            Cell::from(format!("{}", h.port)),
            Cell::from(h.user.as_str()),
        ]).style(row_style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(22),
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Min(12),
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
            Constraint::Min(8),    // Table
            Constraint::Length(2), // Footer instruction
        ])
        .split(inner_area);

    f.render_widget(block, area);
    f.render_widget(table, inner_chunks[0]);

    let footer_p = Paragraph::new(Line::from(Span::styled(
        " (Use ↑/↓ Arrow keys to select | Enter to connect | Esc or Ctrl+O to cancel) ",
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
