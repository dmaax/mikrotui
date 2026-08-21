use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Cell, Row},
    Frame,
};

pub fn render_dhcp_leases(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let leases = app.filtered_dhcp_leases();

    let header_cells = [
        "ID",
        "IP Address",
        "MAC Address",
        "Device Hostname",
        "Server",
        "Status",
        "Expires In",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let selected = app.selection_in(leases.len());

    let rows = leases.iter().enumerate().map(|(idx, item)| {
        let is_selected = Some(idx) == selected;

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
        ])
        .style(row_style)
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "DHCP Server Leases",
        header,
        rows.collect(),
        vec![
            Constraint::Length(4),
            Constraint::Length(18),
            Constraint::Length(20),
            Constraint::Length(25),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(12),
        ],
    );
}
