use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render_system(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let sys = &app.system_resource;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Board & System Info
            Constraint::Length(4), // CPU Gauge
            Constraint::Min(8),    // Memory & Hardware Details
        ])
        .split(area);

    // 1. Board & System Summary
    let sys_info = vec![
        Line::from(vec![
            Span::styled(" Board Model: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.board_name, t.title),
            Span::raw("   |   "),
            Span::styled(" Architecture: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.architecture_name, t.normal_text),
            Span::raw("   |   "),
            Span::styled(" Platform: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.platform, t.normal_text),
        ]),
        Line::from(vec![
            Span::styled(" RouterOS Version: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.version, t.success.add_modifier(Modifier::BOLD)),
            Span::raw("   |   "),
            Span::styled(" Build Date: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.build_time, t.muted_text),
            Span::raw("   |   "),
            Span::styled(" Uptime: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.uptime, t.warning.add_modifier(Modifier::BOLD)),
        ]),
    ];
    let sys_p = Paragraph::new(sys_info)
        .block(Block::default().borders(Borders::ALL).border_style(t.border).title(" Router Information "));
    f.render_widget(sys_p, chunks[0]);

    // 2. CPU Load Gauge
    let cpu_percent = sys.cpu_load.min(100);
    let cpu_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(t.border).title(format!(" CPU Load ({} Cores @ {}) ", sys.cpu_count, sys.cpu_frequency)))
        .gauge_style(if cpu_percent > 85 { t.danger } else if cpu_percent > 60 { t.warning } else { t.success })
        .percent(cpu_percent as u16)
        .label(format!("{}% CPU Usage", cpu_percent));
    f.render_widget(cpu_gauge, chunks[1]);

    // 3. Hardware Details Block
    let hw_info = vec![
        Line::from(vec![
            Span::styled(" CPU Model: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.cpu, t.normal_text),
            Span::raw("   |   "),
            Span::styled(" Cores: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.cpu_count, t.normal_text),
            Span::raw("   |   "),
            Span::styled(" Frequency: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.cpu_frequency, t.normal_text),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Free RAM: ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.free_memory, t.success.add_modifier(Modifier::BOLD)),
            Span::styled(format!("  / Total: {}", sys.total_memory), t.muted_text),
        ]),
        Line::from(vec![
            Span::styled(" Free Storage (HDD): ", t.accent.add_modifier(Modifier::BOLD)),
            Span::styled(&sys.free_hdd_space, t.success.add_modifier(Modifier::BOLD)),
            Span::styled(format!("  / Total: {}", sys.total_hdd_space), t.muted_text),
        ]),
    ];

    let hw_p = Paragraph::new(hw_info)
        .block(Block::default().borders(Borders::ALL).border_style(t.border).title(" Memory & Storage "));
    f.render_widget(hw_p, chunks[2]);
}
