use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use crate::app::App;

pub fn render_dhcp_leases(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let leases = app.filtered_dhcp_leases();

    let header_cells = ["ID", "IP Address", "MAC Address", "Device Hostname", "Server", "Status", "Expires In"]
        .iter()
        .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = leases.iter().enumerate().map(|(idx, item)| {
        let is_selected = idx == app.selected_index;

        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        Row::new(vec![
            Cell::from(item.id.as_str()),
            Cell::from(item.address.as_str()).style(t.success.add_modifier(Modifier::BOLD)),
            Cell::from(item.mac_address.as_str()).style(t.accent),
            Cell::from(item.host_name.as_str()).style(t.warning),
            Cell::from(item.server.as_str()),
            Cell::from(item.status.as_str()).style(t.success),
            Cell::from(item.expires_after.as_str()),
        ]).style(row_style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(18),
            Constraint::Length(20),
            Constraint::Length(25),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(12),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).border_style(t.border).title(format!(" DHCP Server Leases ({}) ", leases.len())));

    f.render_widget(table, area);
}
