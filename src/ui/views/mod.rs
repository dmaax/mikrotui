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
    style::Style,
    widgets::{
        Block, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState,
    },
    Frame,
};

use crate::app::App;

/// How readily a column can be dropped when the terminal is too narrow for all of them.
///
/// Ratatui's answer to a table that does not fit is to shrink every column, which turned
/// `forward` into `forwar` and `192.168.88.0/24` into `192.16` — nine columns of nothing
/// legible. Dropping the least useful ones outright keeps the rest readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Removing this would leave rows the user cannot tell apart. Never dropped.
    Essential,
    /// Worth showing when there is room.
    Useful,
    /// First to go.
    Extra,
}

/// One column of a table.
pub struct Column {
    pub header: &'static str,
    /// Desired width. The last surviving column always flexes to fill what is left.
    pub width: u16,
    pub priority: Priority,
}

impl Column {
    pub const fn new(header: &'static str, width: u16, priority: Priority) -> Self {
        Self {
            header,
            width,
            priority,
        }
    }
}

/// A row's cells, in the same order as the columns.
pub struct TableRow<'a> {
    pub cells: Vec<Cell<'a>>,
    pub style: Style,
}

/// Choose which columns to show in `available` display columns.
///
/// Drops the lowest priority first, and within a priority the rightmost first, since
/// tables here are ordered with the identifying fields on the left. Essential columns are
/// kept even when they do not fit — at that point the terminal is too narrow for the
/// table to say anything, and silently hiding the name would be worse than clipping it.
fn visible_columns(columns: &[Column], available: u16) -> Vec<usize> {
    let mut kept: Vec<usize> = (0..columns.len()).collect();

    let demand = |kept: &[usize]| -> u16 {
        let widths: u16 = kept.iter().map(|i| columns[*i].width).sum();
        widths + kept.len().saturating_sub(1) as u16
    };

    while demand(&kept) > available {
        let victim = kept
            .iter()
            .enumerate()
            .filter(|(_, i)| columns[**i].priority != Priority::Essential)
            .max_by_key(|(pos, i)| (columns[**i].priority, *pos))
            .map(|(pos, _)| pos);

        match victim {
            Some(pos) => {
                kept.remove(pos);
            }
            None => break,
        }
    }

    kept
}

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
    columns: &[Column],
    rows: Vec<TableRow<'a>>,
    header_style: Style,
) {
    let t = &app.theme;
    let total = rows.len();

    let viewport = area.height.saturating_sub(CHROME_ROWS).max(1) as usize;
    app.viewport_rows.set(viewport);

    // Inside the block borders, and leaving room for the scrollbar when there will be one.
    let inner = area.width.saturating_sub(2);
    let usable = if total > viewport {
        inner.saturating_sub(1)
    } else {
        inner
    };

    let kept = visible_columns(columns, usable);

    let widths: Vec<Constraint> = kept
        .iter()
        .enumerate()
        .map(|(position, index)| {
            // The last surviving column absorbs whatever is left over.
            if position + 1 == kept.len() {
                Constraint::Min(columns[*index].width)
            } else {
                Constraint::Length(columns[*index].width)
            }
        })
        .collect();

    let header = Row::new(
        kept.iter()
            .map(|i| Cell::from(columns[*i].header).style(header_style))
            .collect::<Vec<_>>(),
    )
    .height(1)
    .bottom_margin(1);

    let body: Vec<Row> = rows
        .into_iter()
        .map(|row| {
            let TableRow { mut cells, style } = row;
            // Drop the cells whose columns are gone, from the right so the earlier
            // indices stay valid.
            let mut keep = vec![false; cells.len()];
            for i in &kept {
                if let Some(slot) = keep.get_mut(*i) {
                    *slot = true;
                }
            }
            let mut iter = keep.into_iter();
            cells.retain(|_| iter.next().unwrap_or(false));
            Row::new(cells).style(style)
        })
        .collect();

    // Same clamped index the rows used for their highlight, so the row that is
    // highlighted and the row scrolled into view are always the same one.
    let selected = app.selection_in(total);

    // Resolve the offset here rather than letting the widget adjust it internally, so the
    // range in the title matches what is actually on screen in this same frame.
    let offset = visible_offset(app.table_offset.get(), selected, viewport, total);

    let mut state = TableState::default()
        .with_offset(offset)
        .with_selected(selected);

    let hidden = columns.len() - kept.len();
    let table = Table::new(body, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.border)
            .title(table_title(label, offset, total, viewport, hidden)),
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
fn table_title(
    label: &str,
    offset: usize,
    total: usize,
    viewport: usize,
    hidden_columns: usize,
) -> String {
    let rows = if total > viewport {
        let last = (offset + viewport).min(total);
        format!("{}-{last} of {total}", offset + 1)
    } else {
        total.to_string()
    };

    // Say when columns are missing, so a narrow terminal does not look like the whole
    // story. Enter still shows every field.
    if hidden_columns > 0 {
        format!(" {label} ({rows}, {hidden_columns} cols hidden — Enter for all) ")
    } else {
        format!(" {label} ({rows}) ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cols() -> Vec<Column> {
        vec![
            Column::new("ID", 4, Priority::Extra),
            Column::new("Name", 20, Priority::Essential),
            Column::new("Type", 10, Priority::Useful),
            Column::new("MAC", 18, Priority::Extra),
            Column::new("Comment", 15, Priority::Useful),
        ]
    }

    fn names(columns: &[Column], kept: &[usize]) -> Vec<&'static str> {
        kept.iter().map(|i| columns[*i].header).collect()
    }

    #[test]
    fn a_wide_terminal_keeps_every_column() {
        let c = cols();
        assert_eq!(
            names(&c, &visible_columns(&c, 200)),
            vec!["ID", "Name", "Type", "MAC", "Comment"]
        );
    }

    /// Extras go before anything useful, and within a priority the rightmost goes first.
    #[test]
    fn the_least_useful_column_is_dropped_first() {
        let c = cols();
        // Room for everything but the widest extra.
        assert_eq!(
            names(&c, &visible_columns(&c, 60)),
            vec!["ID", "Name", "Type", "Comment"],
            "MAC is the rightmost Extra"
        );
        // Name(20) + Type(10) + Comment(15) plus two separators needs 47.
        assert_eq!(
            names(&c, &visible_columns(&c, 47)),
            vec!["Name", "Type", "Comment"],
            "ID is the remaining Extra"
        );

        // Below that a Useful column has to go, rightmost first.
        assert_eq!(names(&c, &visible_columns(&c, 46)), vec!["Name", "Type"]);
    }

    /// Squeezing every column to four characters is what this replaces; an essential
    /// column is kept even when it no longer fits, because hiding the name would leave
    /// rows the user cannot tell apart.
    #[test]
    fn essential_columns_are_never_dropped() {
        let c = cols();
        assert_eq!(names(&c, &visible_columns(&c, 10)), vec!["Name"]);
        assert_eq!(names(&c, &visible_columns(&c, 1)), vec!["Name"]);
    }

    #[test]
    fn useful_columns_outlast_extras_even_when_wider() {
        let c = vec![
            Column::new("Wide useful", 40, Priority::Useful),
            Column::new("Narrow extra", 3, Priority::Extra),
            Column::new("Name", 10, Priority::Essential),
        ];
        assert_eq!(
            names(&c, &visible_columns(&c, 52)),
            vec!["Wide useful", "Name"]
        );
    }

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
        assert_eq!(table_title("Logs", 0, 3, 20, 0), " Logs (3) ");
        assert_eq!(table_title("Logs", 0, 100, 20, 0), " Logs (1-20 of 100) ");
        assert_eq!(
            table_title("Logs", 80, 100, 20, 0),
            " Logs (81-100 of 100) "
        );

        // Hidden columns are announced, so a narrow terminal does not read as the whole
        // story.
        assert_eq!(
            table_title("Logs", 0, 3, 20, 2),
            " Logs (3, 2 cols hidden — Enter for all) "
        );
    }
}
