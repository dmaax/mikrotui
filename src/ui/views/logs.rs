use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use crate::app::App;

pub fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let logs = app.filtered_logs();

    let header_cells = ["Time", "Topics", "Log Message"]
        .iter()
        .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = logs.iter().enumerate().map(|(idx, item)| {
        let is_selected = idx == app.selected_index;

        let topic_style = if item.topics.contains("error") {
            t.danger
        } else if item.topics.contains("warning") {
            t.warning
        } else if item.topics.contains("ssh") || item.topics.contains("safe-mode") {
            t.success
        } else {
            t.accent
        };

        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        Row::new(vec![
            Cell::from(item.time.as_str()).style(t.muted_text),
            Cell::from(item.topics.as_str()).style(topic_style.add_modifier(Modifier::BOLD)),
            Cell::from(item.message.as_str()),
        ]).style(row_style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(24),
            Constraint::Min(30),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).border_style(t.border).title(format!(" System Logs - Live Stream ({}) ", logs.len())));

    f.render_widget(table, area);
}
