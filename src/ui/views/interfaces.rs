use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    widgets::{Block, Borders, Cell, Row, Sparkline, Table},
    Frame,
};
use crate::app::{format_bps, App};

pub fn render_interfaces(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    // Split view into Top (Interfaces Table) and Bottom (Traffic Sparklines)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),    // Table
            Constraint::Length(8), // Sparklines
        ])
        .split(area);

    let interfaces = app.filtered_interfaces();

    let header_cells = ["ID", "R", "Interface Name", "Type", "MTU", "MAC Address", "Rx Packets", "Tx Packets", "Comment"]
        .iter()
        .map(|h| Cell::from(*h).style(t.header_cell));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = interfaces.iter().enumerate().map(|(idx, i)| {
        let is_selected = idx == app.selected_index;

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
            Cell::from(format!("{}", i.rx_packet)),
            Cell::from(format!("{}", i.tx_packet)),
            Cell::from(i.comment.as_str()).style(t.muted_text),
        ]).style(row_style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(20),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Min(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).border_style(t.border).title(format!(" Network Interfaces ({}) ", interfaces.len())));

    f.render_widget(table, chunks[0]);

    // Bottom Traffic Sparklines Block
    let selected_name = interfaces
        .get(app.selected_index)
        .map(|i| i.name.as_str())
        .unwrap_or("ether1");

    let sparkline_block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_focus)
        .title(format!(" 📈 Real-Time Traffic Sparklines (Interface: {}) ", selected_name));

    let sparkline_inner = Rect {
        x: chunks[1].x + 1,
        y: chunks[1].y + 1,
        width: chunks[1].width.saturating_sub(2),
        height: chunks[1].height.saturating_sub(2),
    };

    let sparkline_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Rx Inbound
            Constraint::Percentage(50), // Tx Outbound
        ])
        .split(sparkline_inner);

    f.render_widget(sparkline_block, chunks[1]);

    // 1. Rx Inbound Sparkline Panel
    let rx_title = format!(" 📥 Rx (Inbound): {} ", format_bps(app.current_rx_bps));
    let rx_block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border)
        .title(rx_title);

    let rx_sparkline = Sparkline::default()
        .block(rx_block)
        .data(&app.rx_history)
        .style(t.success);

    f.render_widget(rx_sparkline, sparkline_chunks[0]);

    // 2. Tx Outbound Sparkline Panel
    let tx_title = format!(" 📤 Tx (Outbound): {} ", format_bps(app.current_tx_bps));
    let tx_block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border)
        .title(tx_title);

    let tx_sparkline = Sparkline::default()
        .block(tx_block)
        .data(&app.tx_history)
        .style(t.title);

    f.render_widget(tx_sparkline, sparkline_chunks[1]);
}
