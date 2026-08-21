use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Cell, Row},
    Frame,
};

pub fn render_ip_addresses(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let addresses = app.filtered_ip_addresses();

    let header_cells = [
        "ID",
        "Flags",
        "IP Address / Mask",
        "Network",
        "Interface",
        "Comment",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

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

        Row::new(vec![
            Cell::from(item.id.as_str()),
            Cell::from(flags).style(t.warning.add_modifier(Modifier::BOLD)),
            Cell::from(item.address.as_str()).style(t.success.add_modifier(Modifier::BOLD)),
            Cell::from(item.network.as_str()),
            Cell::from(item.interface.as_str()).style(t.accent),
            Cell::from(item.comment.as_str()).style(t.muted_text),
        ])
        .style(row_style)
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "IP Addresses - /ip address",
        header,
        rows.collect(),
        vec![
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Length(22),
            Constraint::Length(18),
            Constraint::Length(20),
            Constraint::Min(15),
        ],
    );
}
