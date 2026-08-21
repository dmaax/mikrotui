pub mod dhcp;
pub mod firewall;
pub mod interfaces;
pub mod ip_addresses;
pub mod ip_routes;
pub mod logs;
pub mod neighbors;
pub mod system;

use ratatui::{
    layout::{Constraint, Margin, Rect},
    widgets::{
        Block, Borders, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
    Frame,
};

use crate::app::App;

/// Rows consumed by the block border (top and bottom) plus the header and its margin.
const CHROME_ROWS: u16 = 4;

/// Draw a table the user can scroll through.
///
/// Every view previously built a plain `Table` and passed it to `render_widget`, which
/// draws only the rows that fit and offers no way to reach the rest. On a router with
/// more firewall rules, DHCP leases or log lines than the terminal is tall, everything
/// past the first screenful was unreachable.
///
/// Selection and scroll position now go through `TableState`, and the offset is written
/// back to `App` so the next frame resumes where this one stopped.
pub fn render_scrollable_table<'a>(
    f: &mut Frame,
    app: &App,
    area: Rect,
    label: &str,
    header: Row<'a>,
    rows: Vec<Row<'a>>,
    widths: Vec<Constraint>,
) {
    let t = &app.theme;
    let total = rows.len();

    let viewport = area.height.saturating_sub(CHROME_ROWS).max(1) as usize;
    app.viewport_rows.set(viewport);

    // An out-of-range selection would leave no row highlighted; the filter can shrink the
    // list between a keypress and this frame.
    let selected = total
        .checked_sub(1)
        .map(|last| app.selected_index.min(last));

    // Resolve the offset here rather than letting the widget adjust it internally, so the
    // range in the title matches what is actually on screen in this same frame.
    let offset = visible_offset(app.table_offset.get(), selected, viewport, total);

    let mut state = TableState::default()
        .with_offset(offset)
        .with_selected(selected);

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.border)
            .title(table_title(label, offset, total, viewport)),
    );

    f.render_stateful_widget(table, area, &mut state);
    app.table_offset.set(state.offset());

    if total > viewport {
        let mut scrollbar_state = ScrollbarState::new(total)
            .position(state.offset())
            .viewport_content_length(viewport);

        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .style(t.border),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Smallest scroll adjustment that brings `selected` into view.
fn visible_offset(current: usize, selected: Option<usize>, viewport: usize, total: usize) -> usize {
    let max_offset = total.saturating_sub(viewport);
    let mut offset = current.min(max_offset);

    if let Some(selected) = selected {
        if selected < offset {
            offset = selected;
        } else if selected >= offset + viewport {
            offset = selected + 1 - viewport;
        }
    }

    offset.min(max_offset)
}

/// Title showing the visible range, so it is obvious when a list continues off-screen.
fn table_title(label: &str, offset: usize, total: usize, viewport: usize) -> String {
    if total > viewport {
        let last = (offset + viewport).min(total);
        format!(" {label} ({}-{last} of {total}) ", offset + 1)
    } else {
        format!(" {label} ({total}) ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_offset_when_the_selection_is_already_visible() {
        assert_eq!(visible_offset(10, Some(12), 20, 100), 10);
        assert_eq!(visible_offset(0, Some(0), 20, 100), 0);
    }

    #[test]
    fn scrolls_down_just_enough_to_reveal_the_selection() {
        // Viewport shows 0..=19; selecting 20 should move the window by exactly one row.
        assert_eq!(visible_offset(0, Some(20), 20, 100), 1);
        assert_eq!(visible_offset(0, Some(25), 20, 100), 6);
    }

    #[test]
    fn scrolls_up_to_the_selection() {
        assert_eq!(visible_offset(50, Some(10), 20, 100), 10);
    }

    /// The bug this module exists to fix: the last row of a long list must be reachable.
    #[test]
    fn the_last_row_is_reachable() {
        let offset = visible_offset(0, Some(99), 20, 100);
        assert_eq!(offset, 80);
        assert!(offset + 20 > 99, "row 99 must fall inside the viewport");
    }

    #[test]
    fn never_scrolls_past_the_end() {
        assert_eq!(visible_offset(999, Some(0), 20, 100), 0);
        assert_eq!(visible_offset(999, None, 20, 100), 80);
        // Fewer rows than the viewport: nothing to scroll.
        assert_eq!(visible_offset(5, Some(1), 20, 3), 0);
    }

    #[test]
    fn title_reports_the_visible_range_only_when_it_is_partial() {
        assert_eq!(table_title("Logs", 0, 3, 20), " Logs (3) ");
        assert_eq!(table_title("Logs", 0, 100, 20), " Logs (1-20 of 100) ");
        assert_eq!(table_title("Logs", 80, 100, 20), " Logs (81-100 of 100) ");
    }
}
