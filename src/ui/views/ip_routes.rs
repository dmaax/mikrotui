use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Cell, Row},
    Frame,
};

pub fn render_ip_routes(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let routes = app.filtered_ip_routes();

    let header_cells = [
        "ID",
        "Flags",
        "Dst. Address",
        "Gateway",
        "Distance",
        "Routing Table",
        "Comment",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = routes.iter().enumerate().map(|(idx, item)| {
        let is_selected = idx == app.selected_index;

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

        Row::new(vec![
            Cell::from(item.id.as_str()),
            Cell::from(flags).style(t.accent.add_modifier(Modifier::BOLD)),
            Cell::from(item.dst_address.as_str()).style(t.success.add_modifier(Modifier::BOLD)),
            Cell::from(item.gateway.as_str()).style(t.warning),
            Cell::from(format!("{}", item.distance)),
            Cell::from(item.routing_table.as_str()),
            Cell::from(item.comment.as_str()).style(t.muted_text),
        ])
        .style(row_style)
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "Routing Table - /ip route",
        header,
        rows.collect(),
        vec![
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Length(22),
            Constraint::Length(22),
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Min(15),
        ],
    );
}
