use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::{InputMode, Model};

use super::truncate;
use crate::tui::theme::Palette;

pub(super) fn render_automation(f: &mut Frame, app: &Model, area: Rect) {
    let mut fields = Vec::new();
    let mut index = 0;

    for (monitor_index, monitor) in app.config.monitors.iter().enumerate() {
        if let Some((min_input, max_input)) = app.form.monitor_inputs.get(monitor_index) {
            fields.push((
                format!("{} Min Limit %", truncate(&monitor.logical_id, 16)),
                min_input.value().to_string(),
                index,
            ));
            index += 1;
            fields.push((
                format!("{} Max Limit %", truncate(&monitor.logical_id, 16)),
                max_input.value().to_string(),
                index,
            ));
            index += 1;
        }
    }

    fields.push((String::new(), String::new(), usize::MAX));

    fields.push((
        String::from(" ── Power Management ──"),
        String::from("SUBHEADING"),
        usize::MAX,
    ));

    fields.push((
        String::from("Dim Automatically (Minutes, 0 to disable)"),
        app.form
            .desktop_idle_timeout_minutes_input
            .value()
            .to_string(),
        index,
    ));

    let palette = app.config.tui.theme.palette();
    render_settings_layout(f, app, area, " Brightness Limits ", fields, &palette);
}

pub(super) fn render_location(f: &mut Frame, app: &Model, area: Rect) {
    let palette = app.config.tui.theme.palette();
    render_settings_layout(
        f,
        app,
        area,
        " Solar Location ",
        vec![
            (
                String::from("City"),
                app.form.city_search_input.value().to_string(),
                0,
            ),
            (
                String::from("Latitude"),
                app.form.lat_input.value().to_string(),
                1,
            ),
            (
                String::from("Longitude"),
                app.form.lon_input.value().to_string(),
                2,
            ),
        ],
        &palette,
    );

    // Render city autocomplete popup
    if app.active_setting == 0
        && matches!(app.input_mode, InputMode::Editing)
        && !app.form.city_search_results.is_empty()
    {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.border_inactive))
            .title(" Solar Location ");
        let inner = block.inner(area);
        let popup_area = Rect {
            x: inner.x,
            y: inner.y + 3,
            width: inner.width,
            height: (app.form.city_search_results.len() as u16 + 2).min(12),
        };

        f.render_widget(ratatui::widgets::Clear, popup_area);

        let cities = crate::tui::cities::get_cities();
        let items: Vec<ratatui::widgets::ListItem> = app
            .form
            .city_search_results
            .iter()
            .map(|&idx| {
                let city = &cities[idx];
                let content = format!("{}, {} ({})", city.name, city.country, city.timezone);
                ratatui::widgets::ListItem::new(content)
            })
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(app.form.city_search_selected_index));

        let list = ratatui::widgets::List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.bg))
                    .style(Style::default().bg(palette.bg)),
            )
            .highlight_style(Style::default().bg(palette.accent).fg(palette.bg))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, popup_area, &mut list_state);
    }
}

pub(super) fn render_control(f: &mut Frame, app: &Model, area: Rect) {
    let palette = app.config.tui.theme.palette();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(18),
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(area);

    let control_lines = if let Some(status) = &app.status {
        let until = format_suspend_until(
            status.suspend_until_epoch_s,
            status.suspended,
            &app.config.location.timezone,
            app.config.tui.use_12h_time,
        );

        vec![
            Line::from(format!(" Current suspend : {until}")),
            Line::from(format!(
                " Quick actions : [s] suspend {}   [r] resume now",
                suspend_action_label(app.form.suspend_minutes_input.value())
            )),
        ]
    } else {
        vec![
            Line::from(" Current suspend : unknown"),
            Line::from(format!(
                " Quick actions : [s] suspend {}   [r] resume now",
                suspend_action_label(app.form.suspend_minutes_input.value())
            )),
        ]
    };

    f.render_widget(
        Paragraph::new(control_lines)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.border_inactive))
                    .title(Span::styled(
                        " Daemon Control ",
                        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
                    )),
            ),
        rows[1],
    );

    let inputs_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(rows[0]);

    let fps_is_active = app.active_setting == 1;
    let fps_field_style = active_field_style(app, fps_is_active, &palette);
    let fps_value = format!(" {} ", app.form.fps_input.value());

    let fps_field = Paragraph::new(fps_value)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_border_style(fps_is_active, &palette))
                .title(field_title(
                    "TUI Refresh Rate (FPS)",
                    fps_is_active,
                    &palette,
                )),
        )
        .style(fps_field_style);
    f.render_widget(fps_field, inputs_layout[1]);

    let use_12h_is_active = app.active_setting == 2;
    let use_12h_style = active_field_style(app, use_12h_is_active, &palette);
    let use_12h_value = if app.config.tui.use_12h_time {
        " [x] 12h (AM/PM) "
    } else {
        " [ ] 24h "
    };

    let use_12h_field = Paragraph::new(use_12h_value)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_border_style(use_12h_is_active, &palette))
                .title(field_title(
                    "Time Format (Toggle with Enter)",
                    use_12h_is_active,
                    &palette,
                )),
        )
        .style(use_12h_style);
    f.render_widget(use_12h_field, inputs_layout[2]);

    let unit_is_active = app.active_setting == 3;
    let unit_style = active_field_style(app, unit_is_active, &palette);
    let unit_value = match app.config.tui.temperature_unit {
        crate::config::TemperatureUnit::Celsius => " [x] Celsius  [ ] Fahrenheit ",
        crate::config::TemperatureUnit::Fahrenheit => " [ ] Celsius  [x] Fahrenheit ",
    };

    let unit_field = Paragraph::new(unit_value)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_border_style(unit_is_active, &palette))
                .title(field_title(
                    "Temperature Unit (Toggle with Enter)",
                    unit_is_active,
                    &palette,
                )),
        )
        .style(unit_style);
    f.render_widget(unit_field, inputs_layout[3]);

    let smooth_transition_is_active = app.active_setting == 4;
    let smooth_transition_style = active_field_style(app, smooth_transition_is_active, &palette);
    let smooth_transition_value = if app.config.daemon.smooth_transition {
        " [x] Enabled (multi-step DDC/CI fades) "
    } else {
        " [ ] Disabled (one direct brightness write) "
    };

    let smooth_transition_field = Paragraph::new(smooth_transition_value)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_border_style(smooth_transition_is_active, &palette))
                .title(field_title(
                    "Smooth Brightness Transitions (Toggle with Enter)",
                    smooth_transition_is_active,
                    &palette,
                )),
        )
        .style(smooth_transition_style);
    f.render_widget(smooth_transition_field, inputs_layout[4]);

    let suspend_is_active = app.active_setting == 5;
    let suspend_field_style = active_field_style(app, suspend_is_active, &palette);
    let suspend_value = if matches!(app.input_mode, InputMode::Editing) && suspend_is_active {
        format!(" {} ", app.form.suspend_minutes_input.value())
    } else if app.form.suspend_minutes_input.value().trim().is_empty() {
        String::from(" until resume ")
    } else {
        format!(" {} ", app.form.suspend_minutes_input.value())
    };

    let suspend_field = Paragraph::new(suspend_value)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_border_style(suspend_is_active, &palette))
                .title(field_title(
                    "Suspend Duration Minutes (blank = until resume)",
                    suspend_is_active,
                    &palette,
                )),
        )
        .style(suspend_field_style);
    f.render_widget(suspend_field, inputs_layout[5]);

    let theme_is_active = app.active_setting == 0;
    let theme_style = active_field_style(app, theme_is_active, &palette);
    let theme_value = format!(" [{}] ", app.config.tui.theme.name());

    let theme_field = Paragraph::new(theme_value)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(field_border_style(theme_is_active, &palette))
                .title(field_title(
                    "Theme (Enter to select)",
                    theme_is_active,
                    &palette,
                )),
        )
        .style(theme_style);
    f.render_widget(theme_field, inputs_layout[0]);

    if fps_is_active && matches!(app.input_mode, InputMode::Editing) {
        let cursor_x = inputs_layout[1].x + app.form.fps_input.visual_cursor() as u16 + 2;
        let cursor_y = inputs_layout[1].y + 1;
        f.set_cursor(cursor_x, cursor_y);
    } else if suspend_is_active && matches!(app.input_mode, InputMode::Editing) {
        let cursor_x =
            inputs_layout[5].x + app.form.suspend_minutes_input.visual_cursor() as u16 + 2;
        let cursor_y = inputs_layout[5].y + 1;
        f.set_cursor(cursor_x, cursor_y);
    }
}

pub(super) fn render_settings_layout(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    title: &str,
    fields: Vec<(String, String, usize)>,
    palette: &Palette,
) {
    // Panel border: muted — the tab bar already marks the active section.
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.border_inactive))
        .title(Span::styled(
            title,
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let mut constraints = fields
        .iter()
        .map(|(label, value, _)| {
            if label.is_empty() || value == "SUBHEADING" {
                Constraint::Length(1)
            } else {
                Constraint::Length(3)
            }
        })
        .collect::<Vec<_>>();
    constraints.push(Constraint::Min(0));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (index, (label, value, active_idx)) in fields.iter().enumerate() {
        if label.is_empty() {
            continue;
        }

        if value == "SUBHEADING" {
            let p = Paragraph::new(Span::styled(
                label.as_str(),
                Style::default()
                    .fg(palette.secondary_accent)
                    .add_modifier(Modifier::BOLD),
            ));
            f.render_widget(p, chunks[index]);
            continue;
        }

        let is_active = app.active_setting == *active_idx;
        let style = active_field_style(app, is_active, palette);
        let display_value = display_field_value(label, value, is_active, app);
        let field = Paragraph::new(display_value)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(field_border_style(is_active, palette))
                    .title(field_title(label.as_str(), is_active, palette)),
            )
            .style(style);
        f.render_widget(field, chunks[index]);

        if is_active && matches!(app.input_mode, InputMode::Editing) {
            let Some(active_input) = app.active_input_ref() else {
                continue;
            };
            let cursor_x = chunks[index].x + active_input.visual_cursor() as u16 + 2;
            let cursor_y = chunks[index].y + 1;
            if cursor_x < f.size().width {
                f.set_cursor(cursor_x, cursor_y);
            }
        }
    }

    if let Some(error) = &app.config_error {
        let status = Span::styled(
            format!(" Error: {error}"),
            Style::default().fg(palette.error),
        );
        f.render_widget(
            Paragraph::new(status).wrap(ratatui::widgets::Wrap { trim: true }),
            chunks[fields.len()],
        );
    }
}

/// Border style for individual input fields: brand orange when active/focused,
/// muted when the field is not selected.
fn field_border_style(is_active: bool, palette: &Palette) -> Style {
    if is_active {
        Style::default().fg(palette.secondary_accent)
    } else {
        Style::default().fg(palette.border_inactive)
    }
}

/// Title span for a field: bold white when active, muted when not.
fn field_title<'a>(label: &'a str, is_active: bool, palette: &Palette) -> Span<'a> {
    if is_active {
        Span::styled(
            label,
            Style::default()
                .fg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label, Style::default().fg(palette.border_inactive))
    }
}

fn active_field_style(app: &Model, is_active: bool, palette: &Palette) -> Style {
    if is_active {
        if matches!(app.input_mode, InputMode::Editing) {
            Style::default()
                .fg(palette.bg)
                .bg(palette.warning)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(palette.bg)
                .bg(palette.accent)
                .add_modifier(Modifier::BOLD)
        }
    } else {
        Style::default().fg(palette.fg)
    }
}

fn display_field_value(label: &str, value: &str, is_active: bool, app: &Model) -> String {
    if label.to_lowercase().contains("api key") {
        if matches!(app.input_mode, InputMode::Editing) && is_active {
            if value.is_empty() {
                String::from(" <editing - type your key> ")
            } else {
                format!(" {value} ")
            }
        } else if value.is_empty() {
            String::from(" <not set - press enter to edit> ")
        } else {
            format!(" {} ", "*".repeat(value.len().max(16)))
        }
    } else if label == "City" && value.is_empty() {
        if matches!(app.input_mode, InputMode::Editing) && is_active {
            String::from("  ")
        } else {
            String::from(" <enter your city...> ")
        }
    } else {
        format!(" {value} ")
    }
}

fn format_suspend_until(
    until_epoch_s: Option<u64>,
    suspended: bool,
    timezone: &str,
    use_12h_time: bool,
) -> String {
    if !suspended {
        return String::from("not suspended");
    }

    let Some(epoch_s) = until_epoch_s else {
        return String::from("until resume");
    };

    let Some(dt) = chrono::DateTime::from_timestamp(epoch_s as i64, 0) else {
        return format!("epoch {epoch_s}");
    };

    let path = std::path::Path::new("/usr/share/zoneinfo").join(timezone);
    let tz_offset_and_abbr = std::fs::read(&path)
        .ok()
        .and_then(|data| tz::TimeZone::from_tz_data(&data).ok())
        .or_else(|| tz::TimeZone::from_posix_tz(timezone).ok())
        .and_then(|tz| {
            tz.find_local_time_type(dt.timestamp())
                .ok()
                .map(|lt| (lt.ut_offset(), lt.time_zone_designation().to_string()))
        });

    let absolute = if let Some((offset_secs, abbr)) = tz_offset_and_abbr {
        let offset = chrono::FixedOffset::east_opt(offset_secs).unwrap();
        let format_str = if use_12h_time { "%I:%M %p" } else { "%H:%M" };
        format!("{} {}", dt.with_timezone(&offset).format(format_str), abbr)
    } else {
        let format_str = if use_12h_time {
            "%I:%M %p %Z"
        } else {
            "%H:%M %Z"
        };
        dt.with_timezone(&chrono::Local)
            .format(format_str)
            .to_string()
    };

    let now_epoch_s = chrono::Utc::now().timestamp().max(0) as u64;
    let remaining_minutes = epoch_s.saturating_sub(now_epoch_s).div_ceil(60);
    format!("{absolute} ({remaining_minutes} min left)")
}

fn suspend_action_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        String::from("until resume")
    } else {
        format!("for {trimmed} min")
    }
}
