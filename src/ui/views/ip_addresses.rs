use super::{Column, Priority};
use crate::app::App;
use ratatui::{layout::Rect, style::Modifier, widgets::Cell, Frame};

const COLUMNS: &[Column] = &[
    Column::new("ID", 4, Priority::Extra),
    Column::new("Flags", 6, Priority::Useful),
    Column::new("Address", 22, Priority::Essential),
    Column::new("Network", 18, Priority::Extra),
    Column::new("Interface", 20, Priority::Essential),
    Column::new("Comment", 15, Priority::Useful),
];

pub fn render_ip_addresses(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let addresses = app.filtered_ip_addresses();

    let selected = app.selection_in(addresses.len());

    let rows = addresses.iter().enumerate().map(|(idx, item)| {
        let is_selected = Some(idx) == selected;

        let flags = format!(
            "{}{}",
            if item.dynamic { "D" } else { " " },
            if item.disabled { "X" } else { " " }
        );

        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        super::TableRow {
            cells: vec![
                Cell::from(item.id.as_str()),
                Cell::from(flags).style(t.warning.add_modifier(Modifier::BOLD)),
                Cell::from(item.address.as_str()).style(t.success.add_modifier(Modifier::BOLD)),
                Cell::from(item.network.as_str()),
                Cell::from(item.interface.as_str()).style(t.accent),
                Cell::from(item.comment.as_str()).style(t.muted_text),
            ],
            style: row_style,
        }
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "IP Addresses - /ip address",
        COLUMNS,
        rows.collect(),
        t.header_cell,
    );
}
