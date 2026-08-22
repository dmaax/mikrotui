use super::{Column, Priority};
use crate::app::App;
use ratatui::{layout::Rect, style::Modifier, widgets::Cell, Frame};

const COLUMNS: &[Column] = &[
    Column::new("ID", 4, Priority::Extra),
    Column::new("Interface", 14, Priority::Useful),
    Column::new("Identity", 22, Priority::Essential),
    Column::new("IP Address", 16, Priority::Essential),
    Column::new("MAC Address", 20, Priority::Useful),
    Column::new("Platform", 12, Priority::Extra),
    Column::new("Board / Version", 20, Priority::Useful),
];

pub fn render_neighbors(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let neighbors = app.filtered_neighbors();

    let selected = app.selection_in(neighbors.len());

    let rows = neighbors.iter().enumerate().map(|(idx, n)| {
        let is_selected = Some(idx) == selected;
        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        super::TableRow {
            cells: vec![
                Cell::from(format!("{}", idx + 1)),
                Cell::from(n.interface.as_str()).style(t.accent),
                Cell::from(n.identity.as_str()).style(t.title.add_modifier(Modifier::BOLD)),
                Cell::from(n.ip_address.as_str()).style(t.success),
                Cell::from(n.mac_address.as_str()),
                Cell::from(n.board.as_str()),
                Cell::from(n.version.as_str()).style(t.muted_text),
            ],
            style: row_style,
        }
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "📡 Network Neighbors (MNDP/CDP/LLDP)",
        COLUMNS,
        rows.collect(),
        t.header_cell,
    );
}
