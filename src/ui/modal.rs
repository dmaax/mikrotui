use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use crate::app::{App, Tab};

pub fn render_detail_modal(f: &mut Frame, app: &App) {
    if !app.show_detail_modal {
        return;
    }

    let t = &app.theme;

    // Centered modal area calculation (65% width, 65% height)
    let area = centered_rect(65, 65, f.area());

    // Clear background behind modal
    f.render_widget(Clear, area);

    let (title, mut lines) = get_modal_content(app);

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" (Press Enter, Esc, or 'q' to close this window) ", t.muted_text)));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_focus)
        .title(Span::styled(format!(" 🔍 {} ", title), t.title));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn get_modal_content(app: &App) -> (String, Vec<Line<'static>>) {
    let t = &app.theme;

    match app.active_tab {
        Tab::System => {
            let res = &app.system_resource;
            let lines = vec![
                Line::from(vec![Span::styled("Board Model: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(res.board_name.clone(), t.normal_text)]),
                Line::from(vec![Span::styled("Platform: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(res.platform.clone(), t.normal_text)]),
                Line::from(vec![Span::styled("Architecture: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(res.architecture_name.clone(), t.normal_text)]),
                Line::from(vec![Span::styled("RouterOS Version: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(res.version.clone(), t.normal_text)]),
                Line::from(vec![Span::styled("Build Date: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(res.build_time.clone(), t.normal_text)]),
                Line::from(vec![Span::styled("Uptime: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(res.uptime.clone(), t.warning.add_modifier(Modifier::BOLD))]),
                Line::from(vec![Span::styled("CPU Load: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(format!("{}% ({}x {})", res.cpu_load, res.cpu_count, res.cpu), t.success.add_modifier(Modifier::BOLD))]),
                Line::from(vec![Span::styled("RAM Memory: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(format!("{} free of {}", res.free_memory, res.total_memory), t.normal_text)]),
                Line::from(vec![Span::styled("Storage (HDD): ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(format!("{} free of {}", res.free_hdd_space, res.total_hdd_space), t.normal_text)]),
            ];
            ("System Properties".to_string(), lines)
        }
        Tab::Interfaces => {
            let list = app.filtered_interfaces();
            if let Some(i) = list.get(app.selected_index) {
                let lines = vec![
                    Line::from(vec![Span::styled("Internal ID: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(i.id.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Interface Name: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(i.name.clone(), t.success.add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::styled("Type: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(i.interface_type.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("MTU: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(i.mtu.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("MAC Address: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(i.mac_address.clone(), t.warning)]),
                    Line::from(vec![Span::styled("Running Status: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if i.running { "Yes (Link Up)" } else { "No (Link Down)" }, if i.running { t.success } else { t.danger })]),
                    Line::from(vec![Span::styled("Disabled: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if i.disabled { "Yes" } else { "No" }, t.normal_text)]),
                    Line::from(vec![Span::styled("Received Packets (Rx): ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(format!("{} ({} bytes)", i.rx_packet, i.rx_byte), t.normal_text)]),
                    Line::from(vec![Span::styled("Transmitted Packets (Tx): ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(format!("{} ({} bytes)", i.tx_packet, i.tx_byte), t.normal_text)]),
                    Line::from(""),
                    Line::from(vec![Span::styled("Complete Comment: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if i.comment.is_empty() { "(no comment)".to_string() } else { i.comment.clone() }, t.muted_text)]),
                ];
                (format!("Interface Details [{}]", i.name), lines)
            } else {
                ("Interface Details".to_string(), vec![Line::from("No item selected")])
            }
        }
        Tab::IpAddresses => {
            let list = app.filtered_ip_addresses();
            if let Some(item) = list.get(app.selected_index) {
                let lines = vec![
                    Line::from(vec![Span::styled("Internal ID: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.id.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("IP Address / Mask: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.address.clone(), t.success.add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::styled("Network Subnet: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.network.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Associated Interface: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.interface.clone(), t.accent)]),
                    Line::from(vec![Span::styled("Origin Type: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.dynamic { "Dynamic (DHCP/PPP)" } else { "Static" }, t.warning)]),
                    Line::from(vec![Span::styled("Status: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.disabled { "Disabled" } else { "Active / Enabled" }, if item.disabled { t.danger } else { t.success })]),
                    Line::from(""),
                    Line::from(vec![Span::styled("Complete Comment: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.comment.is_empty() { "(no comment)".to_string() } else { item.comment.clone() }, t.muted_text)]),
                ];
                (format!("IP Address Details [{}]", item.address), lines)
            } else {
                ("IP Address Details".to_string(), vec![Line::from("No item selected")])
            }
        }
        Tab::IpRoutes => {
            let list = app.filtered_ip_routes();
            if let Some(item) = list.get(app.selected_index) {
                let lines = vec![
                    Line::from(vec![Span::styled("Internal ID: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.id.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Destination Address: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.dst_address.clone(), t.success.add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::styled("Gateway: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.gateway.clone(), t.warning)]),
                    Line::from(vec![Span::styled("Administrative Distance: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(format!("{}", item.distance), t.normal_text)]),
                    Line::from(vec![Span::styled("Routing Table: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.routing_table.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Route Status: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.active { "Active" } else { "Inactive" }, if item.active { t.success } else { t.danger })]),
                    Line::from(vec![Span::styled("Origin Type: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.dynamic { "Dynamic" } else { "Static" }, t.normal_text)]),
                    Line::from(""),
                    Line::from(vec![Span::styled("Complete Comment: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.comment.is_empty() { "(no comment)".to_string() } else { item.comment.clone() }, t.muted_text)]),
                ];
                (format!("IP Route Details [{}]", item.dst_address), lines)
            } else {
                ("IP Route Details".to_string(), vec![Line::from("No item selected")])
            }
        }
        Tab::DhcpLeases => {
            let list = app.filtered_dhcp_leases();
            if let Some(item) = list.get(app.selected_index) {
                let lines = vec![
                    Line::from(vec![Span::styled("Internal ID: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.id.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Assigned IP Address: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.address.clone(), t.success.add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::styled("MAC Address: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.mac_address.clone(), t.warning)]),
                    Line::from(vec![Span::styled("Device Hostname: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.host_name.is_empty() { "(unknown)".to_string() } else { item.host_name.clone() }, t.accent)]),
                    Line::from(vec![Span::styled("DHCP Server: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.server.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Lease Status: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.status.clone(), t.success)]),
                    Line::from(vec![Span::styled("Expires In: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.expires_after.clone(), t.normal_text)]),
                ];
                (format!("DHCP Lease Details [{}]", item.address), lines)
            } else {
                ("DHCP Lease Details".to_string(), vec![Line::from("No item selected")])
            }
        }
        Tab::Firewall => {
            let list = app.filtered_firewall_rules();
            if let Some(item) = list.get(app.selected_index) {
                let lines = vec![
                    Line::from(vec![Span::styled("Rule ID: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.id.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Chain: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.chain.clone(), t.accent)]),
                    Line::from(vec![Span::styled("Action: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.action.clone(), if item.action == "accept" { t.success } else { t.danger }.add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::styled("Src. Address: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.src_address.is_empty() { "(any)".to_string() } else { item.src_address.clone() }, t.normal_text)]),
                    Line::from(vec![Span::styled("Dst. Address: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.dst_address.is_empty() { "(any)".to_string() } else { item.dst_address.clone() }, t.normal_text)]),
                    Line::from(vec![Span::styled("Protocol / Dst. Port: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(format!("{} : {}", if item.protocol.is_empty() { "any" } else { &item.protocol }, if item.dst_port.is_empty() { "all" } else { &item.dst_port }), t.warning)]),
                    Line::from(vec![Span::styled("Traffic Counters: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(format!("{} packets / {} bytes", item.packets, item.bytes), t.normal_text)]),
                    Line::from(""),
                    Line::from(vec![Span::styled("Complete Comment: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(if item.comment.is_empty() { "(no comment)".to_string() } else { item.comment.clone() }, t.muted_text)]),
                ];
                (format!("Firewall Rule Details [{}]", item.id), lines)
            } else {
                ("Firewall Details".to_string(), vec![Line::from("No item selected")])
            }
        }
        Tab::Neighbors => {
            let list = app.filtered_neighbors();
            if let Some(item) = list.get(app.selected_index) {
                let lines = vec![
                    Line::from(vec![Span::styled("Internal ID: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.id.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Device Identity: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.identity.clone(), t.title.add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::styled("Local Port/Interface: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.interface.clone(), t.accent)]),
                    Line::from(vec![Span::styled("IP Address: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.ip_address.clone(), t.success.add_modifier(Modifier::BOLD))]),
                    Line::from(vec![Span::styled("MAC Address: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.mac_address.clone(), t.warning)]),
                    Line::from(vec![Span::styled("Board Model: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.board.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("Platform Vendor: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.platform.clone(), t.normal_text)]),
                    Line::from(vec![Span::styled("RouterOS Version: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.version.clone(), t.muted_text)]),
                ];
                (format!("Neighbor Device Details [{}]", item.identity), lines)
            } else {
                ("Neighbor Device Details".to_string(), vec![Line::from("No item selected")])
            }
        }
        Tab::Logs => {
            let list = app.filtered_logs();
            if let Some(item) = list.get(app.selected_index) {
                let lines = vec![
                    Line::from(vec![Span::styled("Event Timestamp: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.time.clone(), t.muted_text)]),
                    Line::from(vec![Span::styled("Log Topics: ", t.accent.add_modifier(Modifier::BOLD)), Span::styled(item.topics.clone(), t.warning.add_modifier(Modifier::BOLD))]),
                    Line::from(""),
                    Line::from(vec![Span::styled("Complete Message: ", t.accent.add_modifier(Modifier::BOLD))]),
                    Line::from(Span::styled(item.message.clone(), t.normal_text)),
                ];
                ("Log Event Details".to_string(), lines)
            } else {
                ("Log Details".to_string(), vec![Line::from("No item selected")])
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
