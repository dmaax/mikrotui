use super::{Column, Priority};
use crate::app::App;
use ratatui::{layout::Rect, style::Modifier, widgets::Cell, Frame};

const COLUMNS: &[Column] = &[
    Column::new("ID", 4, Priority::Extra),
    Column::new("IP Address", 18, Priority::Essential),
    Column::new("MAC Address", 20, Priority::Useful),
    Column::new("Device Hostname", 25, Priority::Essential),
    Column::new("Server", 12, Priority::Extra),
    Column::new("Status", 10, Priority::Useful),
    Column::new("Expires In", 12, Priority::Useful),
];

pub fn render_dhcp_leases(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let leases = app.filtered_dhcp_leases();

    let selected = app.selection_in(leases.len());

    let rows = leases.iter().enumerate().map(|(idx, item)| {
        let is_selected = Some(idx) == selected;

        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        super::TableRow {
            cells: vec![
                Cell::from(item.id.as_str()),
                Cell::from(item.address.as_str()).style(t.success.add_modifier(Modifier::BOLD)),
                Cell::from(item.mac_address.as_str()).style(t.accent),
                Cell::from(item.host_name.as_str()).style(t.warning),
                Cell::from(item.server.as_str()),
                Cell::from(item.status.as_str()).style(t.success),
                Cell::from(item.expires_after.as_str()),
            ],
            style: row_style,
        }
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "DHCP Server Leases",
        COLUMNS,
        rows.collect(),
        t.header_cell,
    );
}
