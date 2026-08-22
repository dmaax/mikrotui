use super::{Column, Priority};
use crate::app::App;
use ratatui::{layout::Rect, style::Modifier, widgets::Cell, Frame};

const COLUMNS: &[Column] = &[
    Column::new("ID", 4, Priority::Extra),
    Column::new("Chain", 10, Priority::Essential),
    Column::new("Action", 10, Priority::Essential),
    Column::new("Src. Address", 16, Priority::Useful),
    Column::new("Dst. Address", 16, Priority::Extra),
    Column::new("Proto", 8, Priority::Useful),
    Column::new("Dst. Port", 14, Priority::Extra),
    Column::new("Bytes", 8, Priority::Extra),
    Column::new("Packets", 8, Priority::Extra),
    Column::new("Comment", 15, Priority::Useful),
];

pub fn render_firewall(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let rules = app.filtered_firewall_rules();

    let selected = app.selection_in(rules.len());

    let rows = rules.iter().enumerate().map(|(idx, item)| {
        let is_selected = Some(idx) == selected;

        let action_style = match item.action.as_str() {
            "accept" => t.success,
            "drop" | "reject" => t.danger,
            _ => t.warning,
        };

        let row_style = if is_selected {
            t.selected_row
        } else {
            t.normal_text
        };

        super::TableRow {
            cells: vec![
                Cell::from(item.id.as_str()),
                Cell::from(item.chain.as_str()).style(t.accent),
                Cell::from(item.action.as_str()).style(action_style.add_modifier(Modifier::BOLD)),
                Cell::from(item.src_address.as_str()),
                Cell::from(item.dst_address.as_str()),
                Cell::from(item.protocol.as_str()),
                Cell::from(item.dst_port.as_str()),
                Cell::from(crate::ui::format::bytes(item.bytes)),
                Cell::from(crate::ui::format::count(item.packets)),
                Cell::from(item.comment.as_str()).style(t.muted_text),
            ],
            style: row_style,
        }
    });

    super::render_scrollable_table(
        f,
        app,
        area,
        "Firewall Filter Rules (Read-Only)",
        COLUMNS,
        rows.collect(),
        t.header_cell,
    );
}
