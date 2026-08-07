pub mod header;
pub mod help_modal;
pub mod modal;
pub mod ping_modal;
pub mod sidebar;
pub mod statusbar;
pub mod theme;
pub mod views;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use crate::app::{App, Tab};

pub fn render(f: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content (Sidebar + Active View)
            Constraint::Length(3), // Statusbar
        ])
        .split(f.area());

    // Header
    header::render_header(f, app, main_chunks[0]);

    // Content split into Sidebar (Left) and Main View (Right)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25), // Sidebar width
            Constraint::Min(40),   // Main view width
        ])
        .split(main_chunks[1]);

    // Sidebar
    sidebar::render_sidebar(f, app, content_chunks[0]);

    // Active View
    match app.active_tab {
        Tab::System => views::system::render_system(f, app, content_chunks[1]),
        Tab::Interfaces => views::interfaces::render_interfaces(f, app, content_chunks[1]),
        Tab::IpAddresses => views::ip_addresses::render_ip_addresses(f, app, content_chunks[1]),
        Tab::IpRoutes => views::ip_routes::render_ip_routes(f, app, content_chunks[1]),
        Tab::DhcpLeases => views::dhcp::render_dhcp_leases(f, app, content_chunks[1]),
        Tab::Firewall => views::firewall::render_firewall(f, app, content_chunks[1]),
        Tab::Logs => views::logs::render_logs(f, app, content_chunks[1]),
    }

    // Statusbar
    statusbar::render_statusbar(f, app, main_chunks[2]);

    // Detail Modal
    modal::render_detail_modal(f, app);

    // Ping Diagnostic Modal
    ping_modal::render_ping_modal(f, app);

    // Help & Keyboard Shortcuts Modal
    help_modal::render_help_modal(f, app);
}
