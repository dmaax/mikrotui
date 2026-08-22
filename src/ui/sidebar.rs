use crate::app::{App, Tab};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

/// Width of the labelled sidebar.
pub const FULL_WIDTH: u16 = 25;

/// Width of the collapsed rail: two digits and a space.
pub const RAIL_WIDTH: u16 = 3;

/// Below this, the labelled sidebar costs more than it is worth.
///
/// At 80 columns it took 25 of them — nearly a third of the screen — for eight labels
/// that never change, while the tables were being squeezed to four characters a column.
pub const COLLAPSE_BELOW: u16 = 100;

/// How much horizontal room the sidebar wants at this terminal width.
pub fn width_for(total: u16) -> u16 {
    if total < COLLAPSE_BELOW {
        RAIL_WIDTH
    } else {
        FULL_WIDTH
    }
}

/// Pad an icon to two display columns.
///
/// The glyphs are not all the same width — `⚙` occupies one column while `🔌` occupies
/// two — so baking spaces into the list left the labels beside them starting at different
/// columns. Measuring is the only way to get this right; counting `char`s cannot, since a
/// wide emoji is one char and two columns.
fn pad_icon(icon: &str) -> String {
    const TARGET: usize = 2;
    let width = icon.width();
    format!("{icon}{}", " ".repeat(TARGET.saturating_sub(width)))
}

pub fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    if area.width < FULL_WIDTH {
        render_rail(f, app, area);
    } else {
        render_full(f, app, area);
    }
}

fn render_full(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let items: Vec<ListItem> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let style = if *tab == app.active_tab {
                t.selected_row
            } else {
                t.normal_text
            };
            // The number is what the rail shows when collapsed, and what the 1-8 keys
            // select, so it is worth showing here too.
            let text = format!(" {} {} {}", index + 1, pad_icon(tab.icon()), tab.title());
            ListItem::new(Line::from(Span::styled(text, style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.border)
            .title(" Menu "),
    );

    f.render_widget(list, area);
}

/// Just the tab numbers, so position among the eight is still visible.
///
/// Icons are deliberately not used here: emoji width varies between terminals, and a rail
/// this narrow has no slack to absorb a cell of drift.
fn render_rail(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let lines: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let style = if *tab == app.active_tab {
                t.selected_row
            } else {
                t.muted_text
            };
            Line::from(Span::styled(format!("{:>2} ", index + 1), style))
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}
