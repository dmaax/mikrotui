pub mod format;
pub mod header;
pub mod help_modal;
pub mod host_switch_modal;
pub mod modal;
pub mod ping_modal;
pub mod sidebar;
pub mod statusbar;
pub mod theme;
pub mod views;

use crate::app::{App, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

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
            Constraint::Min(40),    // Main view width
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
        Tab::Neighbors => views::neighbors::render_neighbors(f, app, content_chunks[1]),
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

    // Quick Host Switcher Modal
    host_switch_modal::render_host_switch_modal(f, app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{LoadedData, Tab};
    use crate::models::Interface;
    use crate::ssh::{RouterClient, SshConfig};
    use ratatui::{backend::TestBackend, Terminal};

    const TERM_WIDTH: u16 = 160;
    const TERM_HEIGHT: u16 = 30;

    fn app_with_interfaces(count: usize) -> App {
        let mut app = App::with_client(RouterClient::new(SshConfig::default()), false);
        app.active_tab = Tab::Interfaces;
        app.interfaces = (0..count)
            .map(|i| Interface {
                name: format!("ether{i}"),
                ..Default::default()
            })
            .collect();
        app
    }

    /// Render one frame and flatten the buffer into text.
    fn screen(app: &App) -> String {
        let backend = TestBackend::new(TERM_WIDTH, TERM_HEIGHT);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, app)).unwrap();

        let buffer = terminal.backend().buffer();
        (0..TERM_HEIGHT)
            .map(|y| {
                (0..TERM_WIDTH)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The defect this feature fixes: with more rows than the terminal is tall, the rows
    /// past the first screenful used to be unreachable no matter what the user pressed.
    #[test]
    fn rows_past_the_first_screenful_are_reachable() {
        let mut app = app_with_interfaces(200);

        // Nothing has scrolled yet: the first row shows, the last does not.
        let top = screen(&app);
        assert!(top.contains("ether0"), "first row should be visible");
        assert!(
            !top.contains("ether199"),
            "last row cannot be on screen before scrolling"
        );

        // Jump to the end, exactly as the End key does.
        app.select_last();
        let bottom = screen(&app);
        assert!(
            bottom.contains("ether199"),
            "last row must be reachable after scrolling to the end"
        );
        assert!(
            !bottom.contains("ether0 "),
            "the view should have scrolled away from the top"
        );
    }

    #[test]
    fn the_title_reports_the_visible_range() {
        let mut app = app_with_interfaces(200);
        assert!(
            screen(&app).contains("of 200"),
            "a truncated list should say how much is off-screen"
        );

        app.select_last();
        assert!(
            screen(&app).contains("200 of 200"),
            "the range should follow the scroll position"
        );
    }

    /// The status message is the only place errors surface. It used to be appended after
    /// ten hotkeys that were already wider than an 80-column terminal, so it never
    /// rendered at exactly the width where something had gone wrong.
    #[test]
    fn the_status_message_survives_a_narrow_terminal() {
        let mut app = app_with_interfaces(3);
        app.status_message = "refresh gave up after 60s".to_string();

        for width in [80u16, 100, 120, 200] {
            let backend = TestBackend::new(width, TERM_HEIGHT);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| render(f, &app)).unwrap();

            let buffer = terminal.backend().buffer();
            let screen: String = (0..TERM_HEIGHT)
                .flat_map(|y| (0..width).map(move |x| (x, y)))
                .map(|(x, y)| buffer[(x, y)].symbol())
                .collect();

            assert!(
                screen.contains("refresh gave up"),
                "the message vanished at {width} columns"
            );
        }
    }

    /// Counts are abbreviated rather than clipped: a byte total cut to its first three
    /// digits reads as a real number, which is worse than showing fewer of them.
    #[test]
    fn large_counts_are_abbreviated_not_clipped() {
        let mut app = App::with_client(RouterClient::new(SshConfig::default()), true);
        app.active_tab = Tab::Firewall;
        app.firewall_rules = vec![crate::models::FirewallRule {
            id: "0".to_string(),
            chain: "input".to_string(),
            action: "accept".to_string(),
            bytes: 194_810_240,
            packets: 1_490_210,
            ..Default::default()
        }];

        let backend = TestBackend::new(200, TERM_HEIGHT);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let buffer = terminal.backend().buffer();
        let screen: String = (0..TERM_HEIGHT)
            .flat_map(|y| (0..200u16).map(move |x| (x, y)))
            .map(|(x, y)| buffer[(x, y)].symbol())
            .collect();

        assert!(
            screen.contains("186Mi"),
            "expected an abbreviated byte count"
        );
        assert!(
            screen.contains("1.49M"),
            "expected an abbreviated packet count"
        );
        assert!(
            !screen.contains("194810240"),
            "the raw value should not be what the column tries to fit"
        );
    }

    /// A scrollbar is the only on-screen cue that a list continues past the viewport.
    #[test]
    fn a_scrollbar_appears_only_when_the_list_overflows() {
        let long = screen(&app_with_interfaces(200));
        assert!(
            long.contains('\u{2588}'),
            "expected a scrollbar thumb for a 200-row list"
        );

        let short = screen(&app_with_interfaces(3));
        assert!(
            !short.contains('\u{2588}'),
            "a list that fits needs no scrollbar"
        );
    }

    /// A list that fits needs no scrolling and should keep the plain count.
    #[test]
    fn short_lists_are_left_alone() {
        let app = app_with_interfaces(3);
        let out = screen(&app);
        assert!(out.contains("ether0") && out.contains("ether2"));
        assert!(out.contains("(3)"), "expected a plain count, got:\n{out}");
    }

    /// A refresh can return fewer rows than the last one. The selection has to come
    /// back inside the list, or the detail modal and the ping target read a row that is
    /// no longer there while a different row appears highlighted.
    #[test]
    fn a_shrinking_refresh_pulls_the_selection_back_into_range() {
        let mut app = app_with_interfaces(200);
        app.select_last();
        assert_eq!(app.selected_index, 199);

        let data = LoadedData {
            interfaces: Some(
                (0..5)
                    .map(|i| Interface {
                        name: format!("ether{i}"),
                        ..Default::default()
                    })
                    .collect(),
            ),
            ..Default::default()
        };
        app.apply_loaded_data(data);

        assert_eq!(
            app.selected_index, 4,
            "selection should land on the last surviving row"
        );
        assert_eq!(app.selection_in(app.interfaces.len()), Some(4));

        // And the row that is highlighted is the one still on screen.
        let out = screen(&app);
        assert!(out.contains("ether4"));
        assert!(!out.contains("ether199"));
    }

    /// The highlight and the scroll position must never disagree about the row.
    #[test]
    fn selection_in_is_clamped_and_empty_safe() {
        let app = app_with_interfaces(10);
        assert_eq!(app.selection_in(10), Some(0));
        assert_eq!(app.selection_in(0), None, "empty list selects nothing");

        let mut app = app_with_interfaces(10);
        app.selected_index = 999;
        assert_eq!(app.selection_in(10), Some(9));
    }

    #[test]
    fn paging_moves_by_a_screenful_and_stops_at_the_ends() {
        let mut app = app_with_interfaces(200);
        screen(&app); // establishes viewport_rows
        let page = app.viewport_rows.get();
        assert!(page > 1, "viewport should hold more than one row");

        app.page_down();
        assert_eq!(app.selected_index, page);

        app.page_up();
        assert_eq!(app.selected_index, 0);

        // Paging up at the top and down at the bottom must not run off either end.
        app.page_up();
        assert_eq!(app.selected_index, 0);
        app.select_last();
        app.page_down();
        assert_eq!(app.selected_index, 199);
    }
}
