use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};
use crate::app::{App, PingState};

pub fn render_ping_modal(f: &mut Frame, app: &App) {
    let t = &app.theme;

    match &app.ping_state {
        PingState::Inactive => {}

        PingState::InputtingTarget { input } => {
            let area = centered_rect(55, 30, f.area());
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(t.border_focus)
                .title(Span::styled(" 📡 Ping Diagnostic Tool (/ping) ", t.title));

            let text = vec![
                Line::from("Enter target IP address or Hostname to test connectivity:"),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" Target IP / Host: ", t.accent.add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" {}█ ", input), t.selected_row),
                ]),
                Line::from(""),
                Line::from(Span::styled(" (Press Enter to start Ping | Esc to cancel) ", t.muted_text)),
            ];

            let p = Paragraph::new(text)
                .block(block)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });

            f.render_widget(p, area);
        }

        PingState::Running { target } => {
            let area = centered_rect(50, 25, f.area());
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(t.border_focus)
                .title(Span::styled(format!(" 📡 Sending Ping to {}... ", target), t.title));

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(" ⏳ Executing 5 ICMP packets via SSH to ", t.normal_text),
                    Span::styled(target, t.success.add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(Span::styled(" Please wait for response from MikroTik... ", t.warning)),
            ];

            let p = Paragraph::new(text)
                .block(block)
                .alignment(Alignment::Center);

            f.render_widget(p, area);
        }

        PingState::Completed { result } => {
            let area = centered_rect(70, 70, f.area());
            f.render_widget(Clear, area);

            let block_main = Block::default()
                .borders(Borders::ALL)
                .border_style(t.border_focus)
                .title(Span::styled(format!(" 📡 Ping Results: {} ", result.target), t.title));

            f.render_widget(block_main, area);

            // Inner content layout
            let inner_area = Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width.saturating_sub(2),
                height: area.height.saturating_sub(2),
            };

            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Stats Bar
                    Constraint::Min(6),    // Table
                    Constraint::Length(2), // Hotkey footer
                ])
                .split(inner_area);

            // Summary Stats Bar
            let loss_style = if result.packet_loss_pct == 0 { t.success } else { t.danger };
            let stats_spans = vec![
                Span::styled(" Sent: ", t.accent),
                Span::styled(format!("{}", result.sent), t.normal_text),
                Span::raw(" | "),
                Span::styled(" Received: ", t.accent),
                Span::styled(format!("{}", result.received), t.success),
                Span::raw(" | "),
                Span::styled(" Loss: ", t.accent),
                Span::styled(format!("{}%", result.packet_loss_pct), loss_style.add_modifier(Modifier::BOLD)),
                Span::raw(" | "),
                Span::styled(" RTT Min/Avg/Max: ", t.accent),
                Span::styled(format!("{}/{}/{} ms", result.min_rtt_ms, result.avg_rtt_ms, result.max_rtt_ms), t.warning.add_modifier(Modifier::BOLD)),
            ];

            let stats_p = Paragraph::new(Line::from(stats_spans))
                .block(Block::default().borders(Borders::BOTTOM).border_style(t.border));
            f.render_widget(stats_p, inner_chunks[0]);

            // Sequence Table
            let header_cells = ["SEQ", "Reply Host", "Size (Bytes)", "TTL", "RTT Time (ms)", "Status"]
                .iter()
                .map(|h| Cell::from(*h).style(t.header_cell));
            let header = Row::new(header_cells).height(1);

            let rows = result.sequences.iter().map(|s| {
                let status_style = if s.status == "ok" || s.rtt_ms > 0 { t.success } else { t.danger };
                Row::new(vec![
                    Cell::from(format!("{}", s.seq)),
                    Cell::from(s.host.as_str()).style(t.accent),
                    Cell::from(format!("{}", s.size)),
                    Cell::from(format!("{}", s.ttl)),
                    Cell::from(format!("{} ms", s.rtt_ms)).style(t.warning.add_modifier(Modifier::BOLD)),
                    Cell::from(s.status.as_str()).style(status_style.add_modifier(Modifier::BOLD)),
                ])
            });

            let table = Table::new(
                rows,
                [
                    Constraint::Length(5),
                    Constraint::Length(22),
                    Constraint::Length(15),
                    Constraint::Length(6),
                    Constraint::Length(15),
                    Constraint::Min(10),
                ],
            )
            .header(header)
            .block(Block::default());

            f.render_widget(table, inner_chunks[1]);

            // Footer instructions
            let footer_p = Paragraph::new(Line::from(Span::styled(
                " (Press Enter, Esc, or 'q' to close this report) ",
                t.muted_text,
            )))
            .alignment(Alignment::Center);

            f.render_widget(footer_p, inner_chunks[2]);
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
