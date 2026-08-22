use crate::app::{App, InputMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Hotkeys, most useful first.
///
/// The order matters: the list is trimmed from the end to whatever room is left after the
/// status message, so whatever sits last is what disappears first on a narrow terminal.
const HOTKEYS: &[(&str, HotkeyTone)] = &[
    ("[?] Help", HotkeyTone::Title),
    ("[q] Quit", HotkeyTone::Danger),
    ("[Tab] Switch", HotkeyTone::Accent),
    ("[/] Search", HotkeyTone::Warning),
    ("[r] Reload", HotkeyTone::Accent),
    ("[Enter] Details", HotkeyTone::Title),
    ("[p] Ping", HotkeyTone::Success),
    ("[PgUp/PgDn] Scroll", HotkeyTone::Accent),
    ("[Ctrl+O] Switch Host", HotkeyTone::Title),
    ("[t] Theme", HotkeyTone::Accent),
];

#[derive(Clone, Copy)]
enum HotkeyTone {
    Accent,
    Title,
    Success,
    Warning,
    Danger,
}

/// One row, no border. See [`crate::ui::header::HEIGHT`].
pub const HEIGHT: u16 = 1;

pub fn render_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    // The filter pane scales with the terminal rather than holding a fixed 35 columns.
    // On an 80-column screen those 35 were a third of the width spent on a hint, while the
    // status message next to it was cut off.
    let filter_width = (area.width / 3).clamp(22, 35);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(filter_width)])
        .split(area);

    let available = chunks[0].width as usize;

    // The message goes first and is never dropped. It is the only place errors surface —
    // a rejected host key, a refresh that gave up — and it used to be appended after ten
    // hotkeys that were already wider than an 80-column terminal, so it never appeared at
    // exactly the moment it mattered.
    let message = app.status_message.as_str();
    let mut spans = vec![Span::styled(format!(" {message}"), t.normal_text)];
    let mut used = message.chars().count() + 1;

    for (label, tone) in HOTKEYS {
        let width = label.chars().count() + 2;
        if used + width + 1 > available {
            break;
        }
        used += width;
        spans.push(Span::styled(
            format!("  {label}"),
            match tone {
                HotkeyTone::Accent => t.accent,
                HotkeyTone::Title => t.title,
                HotkeyTone::Success => t.success,
                HotkeyTone::Warning => t.warning,
                HotkeyTone::Danger => t.danger,
            },
        ));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), chunks[0]);

    // Right filter bar
    let (filter_text, filter_style) = match app.input_mode {
        InputMode::Filtering => (format!(" Search: {}█ ", app.filter_query), t.selected_row),
        InputMode::Normal => {
            if app.filter_query.is_empty() {
                (" Search: (press '/')".to_string(), t.muted_text)
            } else {
                (format!(" Filter: {} ", app.filter_query), t.warning)
            }
        }
    };

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(filter_text, filter_style)))
            .alignment(Alignment::Right),
        chunks[1],
    );
}
