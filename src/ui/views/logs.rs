use super::{Column, Priority};
use crate::app::App;
use ratatui::{layout::Rect, style::Modifier, widgets::Cell, Frame};

const COLUMNS: &[Column] = &[
    Column::new("Time", 12, Priority::Useful),
    Column::new("Topics", 24, Priority::Useful),
    Column::new("Log Message", 30, Priority::Essential),
];

pub fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let logs = app.filtered_logs();

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

        super::TableRow {
            cells: vec![
                Cell::from(item.time.as_str()).style(t.muted_text),
                Cell::from(item.topics.as_str()).style(topic_style.add_modifier(Modifier::BOLD)),
                Cell::from(item.message.as_str()),
            ],
            style: row_style,
        }
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "System Logs - Live Stream",
        COLUMNS,
        rows.collect(),
        t.header_cell,
    );
}
