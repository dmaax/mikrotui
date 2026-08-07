use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use crate::app::{App, Tab};

pub fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let items: Vec<ListItem> = Tab::ALL
        .iter()
        .map(|tab| {
            let is_selected = *tab == app.active_tab;
            let style = if is_selected {
                t.selected_row
            } else {
                t.normal_text
            };

            let text = format!(" {} {}", tab.icon(), tab.title());
            ListItem::new(Line::from(Span::styled(text, style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).border_style(t.border).title(" Menu WinBox "));

    f.render_widget(list, area);
}
