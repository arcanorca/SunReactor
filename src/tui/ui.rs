mod automation;
pub(crate) mod chrome;
pub(crate) mod fonts;
pub(crate) mod kit;
mod light_cycle;
pub(crate) mod location;
pub(crate) mod monitor_selector;
mod monitors;
pub(crate) mod settings;
mod weather;
pub(crate) mod weather_art;
pub(crate) mod weather_model;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::Block,
    Frame,
};

use crate::tui::model::ResponsiveMode;
use crate::tui::{theme::Palette, Model, Tab};

const LOGO_MIN_WIDTH: u16 = 76;
// The full masthead is valuable character, but below this height it steals
// the rows that make the workspaces legible. Compact terminals keep the
// dual-tone `✻ SunReactor` identity row instead.
const LOGO_MIN_HEIGHT: u16 = 32;

pub fn ui(f: &mut Frame, app: &mut Model) {
    let size = f.size();
    let palette = app.config.tui.theme.palette();
    let styles = palette.styles();

    f.render_widget(Block::default().style(styles.base), size);

    let mode = ResponsiveMode::from_size(size.width, size.height);
    if mode == ResponsiveMode::TooSmall {
        render_too_small(f, size, &palette);
        return;
    }

    let show_full_logo =
        app.config.tui.show_logo && size.width >= LOGO_MIN_WIDTH && size.height >= LOGO_MIN_HEIGHT;
    let header_height = if show_full_logo {
        chrome::FULL_HEADER_HEIGHT
    } else {
        1
    };
    // A breathing row above the tabs when the full masthead is shown.
    let header_gap = u16::from(show_full_logo);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(header_gap),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);

    chrome::render_header(f, app, chunks[0], &styles);
    chrome::render_tabs(f, app, chunks[2], &styles);

    let body = body_area(chunks[3]);
    match app.active_tab {
        Tab::Monitors => monitors::render_monitors(f, app, body, &styles),
        Tab::Limits => automation::render_automation(f, app, body, &styles),
        Tab::Location => location::render_location(f, app, body, &styles),
        Tab::Weather => weather::render_weather(f, app, body, &styles),
        Tab::Settings => settings::render_settings(f, app, body, &styles),
    }

    chrome::render_footer(f, app, chunks[4], &styles);

    if app.show_help {
        chrome::render_help(f, app, &palette);
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

/// Workspaces get a one-column gutter and one row of air under the tabs.
fn body_area(area: Rect) -> Rect {
    let top = u16::from(area.height > 12);
    Rect::new(
        area.x + 1,
        area.y + top,
        area.width.saturating_sub(2),
        area.height.saturating_sub(top),
    )
}

fn render_too_small(f: &mut Frame, area: Rect, palette: &Palette) {
    use ratatui::layout::Alignment;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let styles = palette.styles();
    let lines = vec![
        Line::from(vec![
            Span::styled(kit::BRAND_MARK, styles.chrome_title_secondary),
            Span::raw(" "),
            Span::styled("SunReactor", styles.chrome_title),
        ]),
        Line::from(""),
        Line::from(Span::styled("Terminal too small", styles.status_warning)),
        Line::from(Span::styled(
            format!("{}×{} · needs 40×12", area.width, area.height),
            styles.text_muted,
        )),
        Line::from(""),
        Line::from(Span::styled("q to quit", styles.text_muted)),
    ];
    let height = (lines.len() as u16 + 2).min(area.height);
    let inner = kit::centered(area, area.width, height);
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(kit::panel("SunReactor", false, &styles)),
        inner,
    );
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
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(palette.secondary_accent))
                .style(Style::default().bg(palette.bg))
                .title(" Theme · Enter apply · Esc cancel "),
        )
        .highlight_style(
            Style::default()
                .bg(palette.accent)
                .fg(palette.bg)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .highlight_symbol(kit::CURSOR);

    f.render_stateful_widget(list, modal_area, state);
}

pub(super) fn truncate(value: &str, max: usize) -> String {
    use ratatui::text::Span;

    if Span::raw(value).width() <= max {
        value.to_string()
    } else {
        if max == 0 {
            return String::new();
        }

        let mut truncated = String::new();
        let available = max.saturating_sub(Span::raw("…").width());
        for character in value.chars() {
            let mut candidate = truncated.clone();
            candidate.push(character);
            if Span::raw(candidate.as_str()).width() > available {
                break;
            }
            truncated = candidate;
        }
        truncated.push('…');
        truncated
    }
}
