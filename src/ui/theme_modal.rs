use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

/// The theme picker.
///
/// A list rather than the cycle this replaced: 239 themes is not something to step
/// through one keypress at a time. Typing filters, and every movement applies the
/// highlighted theme to the screen behind the modal, so the choice is judged against real
/// tables instead of a swatch.
pub fn render_theme_modal(f: &mut Frame, app: &App) {
    let Some(picker) = app.theme_picker.as_ref() else {
        return;
    };
    let t = &app.theme;

    let area = centered_rect(60, 70, f.area());
    f.render_widget(Clear, area);

    let matches = picker.matches();
    let selected = picker.selected.min(matches.len().saturating_sub(1));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_focus)
        .title(Span::styled(
            format!(" 🎨 Theme ({} of {}) ", matches.len(), picker.entries.len()),
            t.title,
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // search
            Constraint::Min(3),    // list
            Constraint::Length(1), // footer
        ])
        .split(inner);

    let search = if picker.query.is_empty() {
        Span::styled(" Type to search…", t.muted_text)
    } else {
        Span::styled(format!(" {}█", picker.query), t.warning)
    };
    f.render_widget(Paragraph::new(Line::from(search)), chunks[0]);

    let rows = chunks[1].height as usize;
    let offset = visible_offset(picker.offset.get(), selected, rows, matches.len());
    picker.offset.set(offset);

    let items: Vec<ListItem> = matches
        .iter()
        .map(|entry| {
            let current = entry.id == t.id;
            let marker = if current { "● " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(marker, t.success),
                Span::styled(format!("{:<28}", entry.name), t.normal_text),
                Span::styled(format!("{:<10}", entry.source.label()), t.muted_text),
                Span::styled(entry.author.clone(), t.muted_text),
            ]))
        })
        .collect();

    let mut state = ListState::default()
        .with_offset(offset)
        .with_selected(Some(selected));

    f.render_stateful_widget(
        List::new(items).highlight_style(t.selected_row),
        chunks[1],
        &mut state,
    );

    if matches.len() > rows {
        let mut scrollbar_state = ScrollbarState::new(matches.len())
            .position(offset)
            .viewport_content_length(rows);
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

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " ↑↓ move · Enter keep · Esc cancel · Backspace edit ",
            t.muted_text,
        ))),
        chunks[2],
    );
}

/// Smallest scroll that brings `selected` into view. Mirrors the tables' rule.
fn visible_offset(current: usize, selected: usize, viewport: usize, total: usize) -> usize {
    if viewport == 0 {
        return 0;
    }
    let max_offset = total.saturating_sub(viewport);
    let mut offset = current.min(max_offset);
    if selected < offset {
        offset = selected;
    } else if selected >= offset + viewport {
        offset = selected + 1 - viewport;
    }
    offset.min(max_offset)
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
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
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_theme_is_reachable() {
        // 239 entries, 20 visible: selecting the last must scroll to it.
        let offset = visible_offset(0, 238, 20, 239);
        assert_eq!(offset, 219);
        assert!(offset + 20 > 238);
    }

    #[test]
    fn an_empty_list_does_not_panic() {
        assert_eq!(visible_offset(5, 0, 10, 0), 0);
        assert_eq!(visible_offset(0, 0, 0, 100), 0);
    }
}
