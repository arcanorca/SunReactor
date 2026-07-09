mod chrome;
mod monitors;
mod settings;
mod weather;
mod weather_model;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::Block,
    Frame,
};

use crate::tui::{theme::Palette, Model, Tab};

/// Returns a border style for a panel: brand orange when focused, muted when not.
pub(super) fn panel_border(focused: bool, palette: &Palette) -> Style {
    if focused {
        Style::default().fg(palette.border_active)
    } else {
        Style::default().fg(palette.border_inactive)
    }
}

pub fn ui(f: &mut Frame, app: &mut Model) {
    let total_height = f.size().height;
    let palette = app.config.tui.theme.palette();

    f.render_widget(
        Block::default().style(Style::default().bg(palette.bg).fg(palette.fg)),
        f.size(),
    );

    // When the terminal is tall enough (≥ 30 lines) we show the full 5-line
    // ASCII logo (header = 8 lines total). On smaller terminals we collapse to
    // a single-line compact title bar (header = 2 lines) to reclaim space for
    // the actual content panels.
    let header_height: u16 = if total_height >= 30 { 8 } else { 2 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    chrome::render_header(f, app, chunks[0]);
    chrome::render_tabs(f, app, chunks[1]);

    match app.active_tab {
        Tab::Monitors => monitors::render_monitors(f, app, chunks[2]),
        Tab::Limits => settings::render_automation(f, app, chunks[2]),
        Tab::Location => settings::render_location(f, app, chunks[2]),
        Tab::Weather => weather::render_weather(f, app, chunks[2]),
        Tab::Settings => settings::render_control(f, app, chunks[2]),
    }

    chrome::render_footer(f, app, chunks[3]);

    if app.show_help {
        chrome::render_help(f, &palette);
    }

    let mut theme_state = None;
    if let crate::tui::model::ActiveModal::ThemeSelect(ref state, _) = app.active_modal {
        theme_state = Some(state.clone());
    }

    if let Some(mut state) = theme_state {
        render_theme_modal(f, &palette, &mut state);
        if let crate::tui::model::ActiveModal::ThemeSelect(ref mut app_state, _) = app.active_modal
        {
            *app_state = state;
        }
    }
}

fn render_theme_modal(f: &mut Frame, palette: &Palette, state: &mut ratatui::widgets::ListState) {
    use ratatui::widgets::{Borders, Clear, List, ListItem};

    let area = f.size();
    let width = 50;
    let height = 20;
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height.saturating_sub(height) / 2;
    let modal_area =
        ratatui::layout::Rect::new(x, y, width.min(area.width), height.min(area.height));

    f.render_widget(Clear, modal_area);

    let items: Vec<ListItem> = crate::tui::theme::Theme::ALL
        .iter()
        .map(|t| ListItem::new(format!(" {} ", t.name())))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.secondary_accent))
                .style(Style::default().bg(palette.bg))
                .title(" Select Theme (Enter to apply, Esc to cancel) "),
        )
        .highlight_style(
            Style::default()
                .bg(palette.accent)
                .fg(palette.bg)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, modal_area, state);
}

pub(super) fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        let truncated: String = value.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}
