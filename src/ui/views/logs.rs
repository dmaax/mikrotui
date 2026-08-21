use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Cell, Row},
    Frame,
};

pub fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let logs = app.filtered_logs();

    let header_cells = ["Time", "Topics", "Log Message"]
        .iter()
        .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let selected = app.selection_in(logs.len());

    let rows = logs.iter().enumerate().map(|(idx, item)| {
        let is_selected = Some(idx) == selected;

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
        ])
        .style(row_style)
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "System Logs - Live Stream",
        header,
        rows.collect(),
        vec![
            Constraint::Length(12),
            Constraint::Length(24),
            Constraint::Min(30),
        ],
    );
}
