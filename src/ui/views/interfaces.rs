use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use crate::app::App;

pub fn render_interfaces(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let interfaces = app.filtered_interfaces();

    let header_cells = ["ID", "R", "Interface Name", "Type", "MTU", "MAC Address", "Rx Packets", "Tx Packets", "Comment"]
        .iter()
        .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = interfaces.iter().enumerate().map(|(idx, i)| {
        let is_selected = idx == app.selected_index;

        let status = if i.running { "R" } else { " " };
        let status_style = if i.running { t.success } else { t.muted_text };

        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        Row::new(vec![
            Cell::from(i.id.as_str()),
            Cell::from(status).style(status_style.add_modifier(Modifier::BOLD)),
            Cell::from(i.name.as_str()).style(t.accent),
            Cell::from(i.interface_type.as_str()),
            Cell::from(i.mtu.as_str()),
            Cell::from(i.mac_address.as_str()),
            Cell::from(format!("{}", i.rx_packet)),
            Cell::from(format!("{}", i.tx_packet)),
            Cell::from(i.comment.as_str()).style(t.muted_text),
        ]).style(row_style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(20),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Min(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).border_style(t.border).title(format!(" Network Interfaces ({}) ", interfaces.len())));

    f.render_widget(table, area);
}
