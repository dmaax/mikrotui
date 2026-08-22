use super::{Column, Priority};
use crate::app::App;
use ratatui::{layout::Rect, style::Modifier, widgets::Cell, Frame};

const COLUMNS: &[Column] = &[
    Column::new("ID", 4, Priority::Extra),
    Column::new("Flags", 6, Priority::Useful),
    Column::new("Dst. Address", 22, Priority::Essential),
    Column::new("Gateway", 22, Priority::Essential),
    Column::new("Distance", 10, Priority::Useful),
    Column::new("Table", 15, Priority::Extra),
    Column::new("Comment", 15, Priority::Useful),
];

pub fn render_ip_routes(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let routes = app.filtered_ip_routes();

    let selected = app.selection_in(routes.len());

    let rows = routes.iter().enumerate().map(|(idx, item)| {
        let is_selected = Some(idx) == selected;

        let flags = format!(
            "{}{}{}",
            if item.active { "A" } else { " " },
            if item.dynamic { "D" } else { "S" },
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
                Cell::from(flags).style(t.accent.add_modifier(Modifier::BOLD)),
                Cell::from(item.dst_address.as_str()).style(t.success.add_modifier(Modifier::BOLD)),
                Cell::from(item.gateway.as_str()).style(t.warning),
                Cell::from(format!("{}", item.distance)),
                Cell::from(item.routing_table.as_str()),
                Cell::from(item.comment.as_str()).style(t.muted_text),
            ],
            style: row_style,
        }
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "Routing Table - /ip route",
        COLUMNS,
        rows.collect(),
        t.header_cell,
    );
}
