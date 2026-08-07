use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use crate::app::App;

pub fn render_neighbors(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let header_cells = [
        "#",
        "Interface",
        "Device Identity",
        "IP Address",
        "MAC Address",
        "Board / Model",
        "OS Version",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(t.header_cell));

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let neighbors = app.filtered_neighbors();

    let rows = neighbors.iter().enumerate().map(|(idx, n)| {
        let is_selected = idx == app.selected_index;
        let row_style = if is_selected { t.selected_row } else { t.normal_text };

        Row::new(vec![
            Cell::from(format!("{}", idx + 1)),
            Cell::from(n.interface.as_str()).style(t.accent),
            Cell::from(n.identity.as_str()).style(t.title.add_modifier(Modifier::BOLD)),
            Cell::from(n.ip_address.as_str()).style(t.success),
            Cell::from(n.mac_address.as_str()),
            Cell::from(n.board.as_str()),
            Cell::from(n.version.as_str()).style(t.muted_text),
        ])
        .style(row_style)
    });

    let title = format!(" 📡 Network Neighbors (MNDP/CDP/LLDP) [{}] ", neighbors.len());

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),   // #
            Constraint::Length(16),  // Interface
            Constraint::Length(22),  // Identity
            Constraint::Length(18),  // IP Address
            Constraint::Length(20),  // MAC Address
            Constraint::Length(18),  // Board
            Constraint::Min(16),     // Version
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.border_focus)
            .title(title),
    );

    f.render_widget(table, area);
}
