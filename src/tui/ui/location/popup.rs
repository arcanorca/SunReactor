use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use crate::tui::{theme::SemanticStyles, InputMode, Model};

use crate::tui::ui::kit;

/// Renders the city autocomplete suggestions below the City field.
pub(super) fn render_autocomplete_popup(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    styles: &SemanticStyles,
) {
    if app.active_setting != 0
        || !matches!(app.input_mode, InputMode::Editing)
        || app.form.city_search_results.is_empty()
        || area.height < 3
        || area.width < 24
    {
        return;
    }

    let height = (app.form.city_search_results.len() as u16 + 2)
        .min(area.height)
        .min(10);
    // Wide enough for a city and its timezone, without leaving the frame.
    let frame_width = f.size().width;
    let width = area
        .width
        .saturating_sub(2)
        .max(58)
        .min(frame_width.saturating_sub(area.x + 3));
    let popup = Rect::new(area.x + 2, area.y, width, height);
    f.render_widget(Clear, popup);

    let cities = crate::tui::cities::get_cities();
    let name_width = usize::from(popup.width).saturating_sub(6) * 3 / 5;
    let items: Vec<ListItem> = app
        .form
        .city_search_results
        .iter()
        .map(|&index| {
            let city = &cities[index];
            ListItem::new(Line::from(vec![
                Span::styled(
                    kit::pad_to(&format!("{}, {}", city.name, city.country), name_width),
                    styles.text_primary,
                ),
                Span::raw("  "),
                Span::styled(city.timezone.clone(), styles.text_muted),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.form.city_search_selected_index));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles.border_focused)
                .title(Span::styled(" Matches ", styles.text_heading))
                .style(styles.base),
        )
        .highlight_style(styles.item_focused_selected)
        .highlight_symbol(kit::CURSOR);
    f.render_stateful_widget(list, popup, &mut state);
}
