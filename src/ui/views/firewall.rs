use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use crate::app::App;

pub fn render_firewall(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let rules = app.filtered_firewall_rules();

    let header_cells = ["ID", "Chain", "Action", "Src. Address", "Dst. Address", "Proto", "Dst. Port", "Bytes", "Packets", "Comment"]
        .iter()
        .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = rules.iter().enumerate().map(|(idx, item)| {
        let is_selected = idx == app.selected_index;

        let action_style = match item.action.as_str() {
            "accept" => t.success,
            "drop" | "reject" => t.danger,
            _ => t.warning,
        };

        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        Row::new(vec![
            Cell::from(item.id.as_str()),
            Cell::from(item.chain.as_str()).style(t.accent),
            Cell::from(item.action.as_str()).style(action_style.add_modifier(Modifier::BOLD)),
            Cell::from(item.src_address.as_str()),
            Cell::from(item.dst_address.as_str()),
            Cell::from(item.protocol.as_str()),
            Cell::from(item.dst_port.as_str()),
            Cell::from(format!("{}", item.bytes)),
            Cell::from(format!("{}", item.packets)),
            Cell::from(item.comment.as_str()).style(t.muted_text),
        ]).style(row_style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(8),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).border_style(t.border).title(format!(" Firewall Filter Rules (Read-Only) ({}) ", rules.len())));

    f.render_widget(table, area);
}
