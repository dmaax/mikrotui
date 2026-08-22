use super::{Column, Priority};
use crate::app::App;
use ratatui::{layout::Rect, style::Modifier, widgets::Cell, Frame};

const COLUMNS: &[Column] = &[
    Column::new("ID", 4, Priority::Extra),
    Column::new("R", 3, Priority::Essential),
    Column::new("Interface Name", 20, Priority::Essential),
    Column::new("Type", 12, Priority::Useful),
    Column::new("MTU", 8, Priority::Extra),
    Column::new("MAC Address", 20, Priority::Extra),
    Column::new("Rx Pkts", 8, Priority::Useful),
    Column::new("Tx Pkts", 8, Priority::Useful),
    Column::new("Comment", 15, Priority::Useful),
];

pub fn render_interfaces(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let interfaces = app.filtered_interfaces();

    let selected = app.selection_in(interfaces.len());

    let rows = interfaces.iter().enumerate().map(|(idx, i)| {
        let is_selected = Some(idx) == selected;

        let status = if i.running { "R" } else { " " };
        let status_style = if i.running { t.success } else { t.muted_text };

        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        super::TableRow {
            cells: vec![
                Cell::from(i.id.as_str()),
                Cell::from(status).style(status_style.add_modifier(Modifier::BOLD)),
                Cell::from(i.name.as_str()).style(t.accent),
                Cell::from(i.interface_type.as_str()),
                Cell::from(i.mtu.as_str()),
                Cell::from(i.mac_address.as_str()),
                Cell::from(crate::ui::format::count(i.rx_packet)),
                Cell::from(crate::ui::format::count(i.tx_packet)),
                Cell::from(i.comment.as_str()).style(t.muted_text),
            ],
            style: row_style,
        }
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "Network Interfaces",
        COLUMNS,
        rows.collect(),
        t.header_cell,
    );
}
