use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Cell, Row},
    Frame,
};

pub fn render_interfaces(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let interfaces = app.filtered_interfaces();

    let header_cells = [
        "ID",
        "R",
        "Interface Name",
        "Type",
        "MTU",
        "MAC Address",
        "Rx Pkts",
        "Tx Pkts",
        "Comment",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

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

        Row::new(vec![
            Cell::from(i.id.as_str()),
            Cell::from(status).style(status_style.add_modifier(Modifier::BOLD)),
            Cell::from(i.name.as_str()).style(t.accent),
            Cell::from(i.interface_type.as_str()),
            Cell::from(i.mtu.as_str()),
            Cell::from(i.mac_address.as_str()),
            Cell::from(crate::ui::format::count(i.rx_packet)),
            Cell::from(crate::ui::format::count(i.tx_packet)),
            Cell::from(i.comment.as_str()).style(t.muted_text),
        ])
        .style(row_style)
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "Network Interfaces",
        header,
        rows.collect(),
        vec![
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(15),
        ],
    );
}
