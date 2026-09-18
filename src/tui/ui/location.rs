use std::time::Instant;

use chrono::Utc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Circle, Points},
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph,
    },
    Frame,
};

use crate::policy::AutomationMilestone;
use crate::tui::globe::{
    center_for_motion, daylight_at, fit_globe_viewport, is_land, project_orthographic,
    unproject_orthographic, Daylight, GlobeCenter,
};
use crate::tui::{theme::SemanticStyles, InputMode, Model};

use super::kit::{self, FieldKind, FieldState};

const LABEL_WIDTH: usize = 9;

/// Formats a signed latitude as a hemisphere string, e.g. `41.01384° N`.
#[must_use]
pub fn format_latitude_hemisphere(lat: f64) -> String {
    let hemi = if lat >= 0.0 { 'N' } else { 'S' };
    format!("{:.5}° {}", lat.abs(), hemi)
}

/// Formats a signed longitude as a hemisphere string, e.g. `28.94966° E`.
#[must_use]
pub fn format_longitude_hemisphere(lon: f64) -> String {
    let hemi = if lon >= 0.0 { 'E' } else { 'W' };
    format!("{:.5}° {}", lon.abs(), hemi)
}

/// Renders the Location workspace (Tab 3).
pub fn render_location(f: &mut Frame, app: &mut Model, area: Rect, styles: &SemanticStyles) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    if area.width >= 80 && area.height >= 12 {
        let form_width = form_column_width(app).min(area.width / 2);
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(form_width),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        let left = columns[0];
        let form_height = 6u16.min(left.height);
        let form_area = Rect::new(left.x, left.y, left.width, form_height);
        let sun_area = Rect::new(
            left.x,
            left.y + form_height + 1,
            left.width,
            left.height.saturating_sub(form_height + 1).min(7),
        );
        render_location_form(f, app, form_area, styles);
        render_sun_panel(f, app, sun_area, styles);
        render_earth_panel(f, app, columns[2], styles);
        render_autocomplete_popup(
            f,
            app,
            Rect::new(
                left.x,
                left.y + 2,
                left.width.max(40).min(area.width),
                left.height.saturating_sub(2),
            ),
            styles,
        );
    } else {
        let form_height = 6u16.min(area.height);
        let form_area = Rect::new(area.x, area.y, area.width, form_height);
        render_location_form(f, app, form_area, styles);
        let rest = Rect::new(
            area.x,
            area.y + form_height,
            area.width,
            area.height.saturating_sub(form_height),
        );
        if rest.height >= 8 {
            render_earth_panel(f, app, rest, styles);
        } else {
            render_sun_panel(f, app, rest, styles);
        }
        render_autocomplete_popup(
            f,
            app,
            Rect::new(
                area.x,
                area.y + 2,
                area.width,
                area.height.saturating_sub(2),
            ),
            styles,
        );
    }
}

/// The form column only needs room for its longest value and a capsule.
fn form_column_width(app: &Model) -> u16 {
    let values = [
        app.config.location.city.as_str(),
        app.config.location.timezone.as_str(),
        "41.01384° N ",
    ];
    let longest = values
        .iter()
        .map(|value| kit::cell_width(value))
        .max()
        .unwrap_or(12);
    // cursor + label + gap + capsule padding + panel borders and padding.
    (2 + LABEL_WIDTH + 1 + 4 + longest + 4).clamp(34, 46) as u16
}

fn render_location_form(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    let editing = matches!(app.input_mode, InputMode::Editing);
    let block = kit::panel("Location", app.workspace_focused(), styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let value_width = usize::from(inner.width).saturating_sub(2 + LABEL_WIDTH + 1 + 4);
    let city = if app.config.location.city.trim().is_empty() {
        String::from("Not set")
    } else {
        app.config.location.city.clone()
    };
    let timezone = if app.config.location.timezone.trim().is_empty() {
        String::from("UTC")
    } else {
        app.config.location.timezone.clone()
    };
    let fields = [
        ("City", city, app.form.city_search_input.value().to_owned()),
        (
            "Latitude",
            format_latitude_hemisphere(app.config.location.latitude),
            app.form.lat_input.value().to_owned(),
        ),
        (
            "Longitude",
            format_longitude_hemisphere(app.config.location.longitude),
            app.form.lon_input.value().to_owned(),
        ),
        (
            "Timezone",
            timezone,
            app.form.timezone_input.value().to_owned(),
        ),
    ];

    let mut lines = Vec::new();
    for (index, (label, value, edit)) in fields.iter().enumerate() {
        let focused = app.active_setting == index && app.workspace_focused();
        lines.push(kit::field_line(
            label,
            LABEL_WIDTH,
            &super::truncate(value, value_width),
            FieldKind::Text,
            FieldState::new(focused, focused && editing),
            Some(&super::truncate(edit, value_width)),
            styles,
        ));
    }
    f.render_widget(Paragraph::new(lines), inner);

    if editing && app.active_setting < 4 {
        if let Some(input) = app.active_input_ref() {
            let row = Rect::new(inner.x, inner.y + app.active_setting as u16, inner.width, 1);
            kit::place_field_cursor(f, row, LABEL_WIDTH, input.visual_cursor());
        }
    }

    if let Some(error) = &app.config_error {
        if area.y + area.height < f.size().height {
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!(
                        " ✕ {}",
                        super::truncate(error, usize::from(area.width.saturating_sub(4)))
                    ),
                    styles.status_error,
                )),
                Rect::new(area.x, area.y + area.height, area.width, 1),
            );
        }
    }
}

/// Read-only solar facts for the configured location, all taken from the
/// daemon status or the shared milestone schedule.
fn render_sun_panel(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let block = kit::panel("Sun today", false, styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);

    let use_12h = app.config.tui.use_12h_time;
    let format = |time: chrono::DateTime<chrono::FixedOffset>| {
        if use_12h {
            time.format("%I:%M %p").to_string()
        } else {
            time.format("%H:%M").to_string()
        }
    };
    let status = app.status.as_ref();
    let sunrise = status
        .and_then(|status| status.sunrise_epoch_s)
        .and_then(|epoch| app.local_time_at_epoch(epoch));
    let sunset = status
        .and_then(|status| status.sunset_epoch_s)
        .and_then(|epoch| app.local_time_at_epoch(epoch));
    let noon = app
        .selected_monitor_schedule()
        .or_else(|| app.monitor_milestones.first())
        .and_then(|schedule| {
            schedule
                .milestones
                .iter()
                .find(|milestone| milestone.milestone == AutomationMilestone::Peak)
        })
        .map(|milestone| milestone.base_time_local);
    let dash = || vec![Span::styled("—", styles.text_muted)];
    let value = |text: String| vec![Span::styled(text, styles.text_primary)];

    let mut lines = vec![
        kit::data_line(
            "Sunrise",
            LABEL_WIDTH + 1,
            sunrise.map_or_else(dash, |t| value(format(t))),
            styles,
        ),
        kit::data_line(
            "Solar noon",
            LABEL_WIDTH + 1,
            noon.map_or_else(dash, |t| value(format(t))),
            styles,
        ),
        kit::data_line(
            "Sunset",
            LABEL_WIDTH + 1,
            sunset.map_or_else(dash, |t| value(format(t))),
            styles,
        ),
    ];
    if let (Some(rise), Some(set)) = (sunrise, sunset) {
        let minutes = (set - rise).num_minutes().max(0);
        lines.push(kit::data_line(
            "Daylight",
            LABEL_WIDTH + 1,
            value(format!("{}h {:02}m", minutes / 60, minutes % 60)),
            styles,
        ));
    }
    if let Some(elevation) = status.and_then(|status| status.solar_elevation) {
        lines.push(kit::data_line(
            "Sun now",
            LABEL_WIDTH + 1,
            value(format!("{elevation:+.1}°")),
            styles,
        ));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

#[allow(clippy::too_many_lines)]
fn render_earth_panel(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.height < 5 || area.width < 16 {
        return;
    }
    let target = GlobeCenter::new(app.config.location.longitude, app.config.location.latitude);
    let subsolar = crate::solar::ephemeris::subsolar_point_utc(Utc::now());
    let local_light = daylight_at(target.lon_deg, target.lat_deg, subsolar);
    let block = kit::with_meta(
        kit::panel("Earth", false, styles),
        vec![Span::styled(
            match local_light {
                Daylight::Day => "day at location",
                Daylight::Twilight => "twilight at location",
                Daylight::Night => "night at location",
            },
            styles.text_muted,
        )],
    );
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.height < 3 || inner.width < 10 {
        return;
    }

    let legend_rows = u16::from(inner.height >= 10);
    let globe_space = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(legend_rows),
    );
    let viewport = fit_globe_viewport(globe_space, app.cell_metrics.cell_aspect);
    if viewport.width < 12 || viewport.height < 5 {
        return;
    }

    let now = Instant::now();
    let center = center_for_motion(&app.motion, target, now);
    let layers = shade_globe(viewport, center, subsolar);

    let marker = project_orthographic(target.lon_deg, target.lat_deg, center)
        .filter(|point| point.z > 0.0)
        .map(|point| (point.x, point.y));
    let sun = project_orthographic(subsolar.1, subsolar.0, center)
        .filter(|point| point.z > 0.15)
        .map(|point| (point.x, point.y));
    let pulse = app
        .motion
        .location_acquisition_phase(now)
        .map(|(phase, _, _)| phase)
        .or_else(|| {
            app.motion
                .globe_rotation_phase(now)
                .map(|(phase, ..)| phase)
        });

    let palette = styles.palette;
    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .background_color(palette.bg)
        .x_bounds([-1.02, 1.02])
        .y_bounds([-1.02, 1.02])
        .paint(|ctx| {
            ctx.draw(&Points {
                coords: &layers.graticule,
                color: palette.border_inactive,
            });
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 1.0,
                color: palette.border_inactive,
            });
            ctx.layer();
            ctx.draw(&Points {
                coords: &layers.land_night,
                color: palette.border_inactive,
            });
            ctx.draw(&Points {
                coords: &layers.land_twilight,
                color: palette.text_muted,
            });
            ctx.draw(&Points {
                coords: &layers.land_limb,
                color: palette.text_muted,
            });
            ctx.draw(&Points {
                coords: &layers.land_day,
                color: palette.secondary_accent,
            });
            ctx.layer();
            if let Some(phase) = pulse {
                if let Some((x, y)) = marker {
                    ctx.draw(&Circle {
                        x,
                        y,
                        radius: 0.05 + f64::from(phase) * 0.22,
                        color: palette.accent,
                    });
                }
            }
            if let Some((x, y)) = sun {
                ctx.print(x, y, Span::styled(kit::BRAND_MARK, styles.status_warning));
            }
            if let Some((x, y)) = marker {
                ctx.print(
                    x,
                    y,
                    Span::styled(kit::DOT, styles.focus_marker.add_modifier(Modifier::BOLD)),
                );
            }
        });
    f.render_widget(canvas, viewport);

    if legend_rows > 0 {
        let city = if app.config.location.city.trim().is_empty() {
            String::from("Location")
        } else {
            app.config.location.city.clone()
        };
        let legend = Line::from(vec![
            Span::styled(format!("{} ", kit::DOT), styles.focus_marker),
            Span::styled(city, styles.text_primary),
            Span::raw("    "),
            Span::styled(format!("{} ", kit::BRAND_MARK), styles.status_warning),
            Span::styled("sun overhead", styles.text_muted),
            Span::raw("    "),
            Span::styled("⣿ ", styles.text_muted),
            Span::styled("dim land is night", styles.text_muted),
        ]);
        f.render_widget(
            Paragraph::new(legend).alignment(ratatui::layout::Alignment::Center),
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        );
    }
}

struct GlobeLayers {
    graticule: Vec<(f64, f64)>,
    land_day: Vec<(f64, f64)>,
    land_limb: Vec<(f64, f64)>,
    land_twilight: Vec<(f64, f64)>,
    land_night: Vec<(f64, f64)>,
}

/// Samples every Braille dot of the viewport once: land or ocean, lit or not,
/// and near the limb or facing the viewer.
fn shade_globe(viewport: Rect, center: GlobeCenter, subsolar: (f64, f64)) -> GlobeLayers {
    let dots_x = usize::from(viewport.width) * 2;
    let dots_y = usize::from(viewport.height) * 4;
    let span = 2.04;
    let mut layers = GlobeLayers {
        graticule: graticule(center, dots_x.max(dots_y)),
        land_day: Vec::new(),
        land_limb: Vec::new(),
        land_twilight: Vec::new(),
        land_night: Vec::new(),
    };
    for row in 0..dots_y {
        let y = 1.02 - (row as f64 + 0.5) * span / dots_y as f64;
        for column in 0..dots_x {
            let x = -1.02 + (column as f64 + 0.5) * span / dots_x as f64;
            let Some((lon, lat, z)) = unproject_orthographic(x, y, center) else {
                continue;
            };
            let light = daylight_at(lon, lat, subsolar);
            if is_land(lon, lat) {
                match light {
                    Daylight::Day if z < 0.28 => layers.land_limb.push((x, y)),
                    Daylight::Day => layers.land_day.push((x, y)),
                    Daylight::Twilight => layers.land_twilight.push((x, y)),
                    Daylight::Night => layers.land_night.push((x, y)),
                }
            }
        }
    }
    layers
}

/// Dotted 30° meridians and parallels. Their curvature is what makes the
/// orthographic disk read as a sphere.
fn graticule(center: GlobeCenter, resolution: usize) -> Vec<(f64, f64)> {
    let steps = (resolution / 3).clamp(24, 90);
    let mut points = Vec::new();
    let mut push = |lon: f64, lat: f64| {
        if let Some(point) = project_orthographic(lon, lat, center) {
            if point.z > 0.05 {
                points.push((point.x, point.y));
            }
        }
    };
    for latitude in [-60.0, -30.0, 0.0, 30.0, 60.0] {
        for step in 0..steps * 2 {
            push(-180.0 + 360.0 * step as f64 / (steps * 2) as f64, latitude);
        }
    }
    for meridian in 0..12 {
        let longitude = -180.0 + 30.0 * f64::from(meridian);
        for step in 0..=steps {
            push(longitude, -90.0 + 180.0 * step as f64 / steps as f64);
        }
    }
    points
}

/// Renders the city autocomplete suggestions below the City field.
fn render_autocomplete_popup(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use ratatui::{backend::TestBackend, Terminal};

    use crate::tui::{test_support::find_in_buffer, ui, InputMode, Model, Tab};
    #[test]
    fn test_location_workspace_comfortable_form_and_globe() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul, TR");
        model.config.location.latitude = 41.01384;
        model.config.location.longitude = 28.94966;
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());

        let buffer = terminal.backend().buffer().clone();

        // Headers & Fields
        assert!(find_in_buffer(&buffer, "Location").is_some());
        assert!(find_in_buffer(&buffer, "Earth").is_some());
        assert!(find_in_buffer(&buffer, "City").is_some());
        assert!(find_in_buffer(&buffer, "Latitude").is_some());
        assert!(find_in_buffer(&buffer, "Longitude").is_some());
        assert!(find_in_buffer(&buffer, "Timezone").is_some());

        // Hemisphere formatted display values
        assert!(find_in_buffer(&buffer, "41.01384° N").is_some());
        assert!(find_in_buffer(&buffer, "28.94966° E").is_some());
        assert!(find_in_buffer(&buffer, "Europe/Istanbul").is_some());
    }

    #[test]
    fn test_location_globe_metadata_reflows_without_clipping_in_medium_viewport() {
        let backend = TestBackend::new(80, 32);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul, TR");
        model.config.location.latitude = 41.01384;
        model.config.location.longitude = 28.94966;
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Earth").is_some());
        assert!(find_in_buffer(&buffer, "Istanbul, TR").is_some());
        assert!(find_in_buffer(&buffer, "Europe/Istanbul").is_some());
    }

    #[test]
    fn test_location_workspace_western_and_southern_hemispheres() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        // Southern + Eastern (Sydney)
        model.config.location.city = String::from("Sydney, AU");
        model.config.location.latitude = -33.8688;
        model.config.location.longitude = 151.2093;
        model.config.location.timezone = String::from("Australia/Sydney");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "33.86880° S").is_some());
        assert!(find_in_buffer(&buffer, "151.20930° E").is_some());

        // Northern + Western (New York)
        model.config.location.city = String::from("New York, US");
        model.config.location.latitude = 40.7128;
        model.config.location.longitude = -74.0060;
        model.config.location.timezone = String::from("America/New_York");
        model.form.refresh_from_config(&model.config);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer2 = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer2, "40.71280° N").is_some());
        assert!(find_in_buffer(&buffer2, "74.00600° W").is_some());

        // Extreme boundaries (±90, ±180)
        assert_eq!(
            crate::tui::ui::location::format_latitude_hemisphere(90.0),
            "90.00000° N"
        );
        assert_eq!(
            crate::tui::ui::location::format_latitude_hemisphere(-90.0),
            "90.00000° S"
        );
        assert_eq!(
            crate::tui::ui::location::format_longitude_hemisphere(180.0),
            "180.00000° E"
        );
        assert_eq!(
            crate::tui::ui::location::format_longitude_hemisphere(-180.0),
            "180.00000° W"
        );
    }

    #[test]
    fn test_location_workspace_focused_and_editing_city() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul, TR");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;
        model.active_setting = 0; // City field focused

        // Normal mode: ❯ cursor on City, committed value shown
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "❯ City").is_some());
        assert!(find_in_buffer(&buffer, "Istanbul, TR").is_some());

        // Switch to editing mode with new query "Lond"
        model.input_mode = InputMode::Editing;
        model.form.city_search_input = tui_input::Input::default().with_value(String::from("Lond"));
        model.form.city_search_results = vec![0, 1]; // Mock results
        model.form.city_search_selected_index = 0;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_edit = terminal.backend().buffer().clone();

        // The working buffer has its own editing capsule, not concatenated with "Istanbul".
        assert!(find_in_buffer(&buffer_edit, "│ Lond").is_some());
        assert!(find_in_buffer(&buffer_edit, "Istanbul, TRLond").is_none());
        assert!(find_in_buffer(&buffer_edit, "Matches").is_some());
    }

    #[test]
    fn test_location_workspace_unicode_and_cjk_city_input() {
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.input_mode = InputMode::Editing;
        model.active_setting = 0;

        for unicode_city in ["İstanbul", "Zürich", "São Paulo", "Łódź", "東京"] {
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();

            model.form.city_search_input =
                tui_input::Input::default().with_value(String::from(unicode_city));
            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok(), "Failed on city: {unicode_city}");

            let buffer = terminal.backend().buffer().clone();
            if unicode_city == "東京" {
                // In Ratatui, 2-column wide CJK characters occupy 2 cells: (char, continuation space).
                assert!(find_in_buffer(&buffer, "東").is_some());
                assert!(find_in_buffer(&buffer, "京").is_some());
            } else {
                assert!(
                    find_in_buffer(&buffer, unicode_city).is_some(),
                    "Missing Unicode text {unicode_city} in buffer"
                );
            }
        }
    }

    #[test]
    fn test_location_workspace_responsive_compact_and_minimal() {
        // Compact layout (65x22): stacked form + globe
        {
            let backend = TestBackend::new(65, 22);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.active_tab = Tab::Location;

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok());

            let buffer = terminal.backend().buffer().clone();
            assert!(find_in_buffer(&buffer, "Location").is_some());
            assert!(find_in_buffer(&buffer, "Earth").is_some());
        }

        // Minimal layout (50x16): the globe is omitted and the fields stay readable
        {
            let backend = TestBackend::new(50, 16);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.active_tab = Tab::Location;

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok());

            let buffer = terminal.backend().buffer().clone();
            assert!(find_in_buffer(&buffer, "Location").is_some());
            assert!(find_in_buffer(&buffer, "Latitude").is_some());
            assert!(find_in_buffer(&buffer, "Earth").is_none());
        }

        // Strict Minimal layout (45x12): map omitted, essential fields fit without panic
        {
            let backend = TestBackend::new(45, 12);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.active_tab = Tab::Location;

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok());

            let buffer = terminal.backend().buffer().clone();
            assert!(find_in_buffer(&buffer, "Location").is_some());
            assert!(find_in_buffer(&buffer, "City").is_some());
        }
    }

    #[test]
    fn test_location_workspace_non_amber_themes() {
        for theme in [
            crate::config::Theme::Nord,
            crate::config::Theme::HackerGreen,
        ] {
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.theme = theme;
            model.active_tab = Tab::Location;
            model.active_setting = 1; // Latitude focused

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok());

            let buffer = terminal.backend().buffer().clone();
            let marker = find_in_buffer(&buffer, "❯").expect("focus marker present");
            assert_eq!(marker.2.fg, theme.palette().accent);
        }
    }

    #[test]
    fn test_phase5_5_2_city_clean_edit_entry_and_cancel_restore() {
        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul, TR");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;
        model.active_setting = 0; // City

        assert_eq!(model.form.city_search_input.value(), "Istanbul, TR");

        // Enter edit mode on City: must start with clean empty buffer for searching
        model.start_editing();
        assert_eq!(model.input_mode, InputMode::Editing);
        assert_eq!(model.form.city_search_input.value(), "");
        assert!(model.form.city_search_results.is_empty());

        // User cancels editing: must restore prior committed city
        model.cancel_editing();
        assert_eq!(model.input_mode, InputMode::Normal);
        assert_eq!(model.form.city_search_input.value(), "Istanbul, TR");
        assert_eq!(model.config.location.city, "Istanbul, TR");
    }

    #[test]
    fn test_phase5_5_2_location_globe_projection_and_hierarchy() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.location.city = String::from("Tokyo, JP");
        model.config.location.latitude = 35.6895;
        model.config.location.longitude = 139.6917;
        model.config.location.timezone = String::from("Asia/Tokyo");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Section 1: Location Form
        assert!(find_in_buffer(&buffer, "Location").is_some());
        assert!(find_in_buffer(&buffer, "City").is_some());
        assert!(find_in_buffer(&buffer, "Tokyo, JP").is_some());

        // Coordinates are plain fields in the same form.
        assert!(find_in_buffer(&buffer, "35.68950° N").is_some());
        assert!(find_in_buffer(&buffer, "139.69170° E").is_some());
        assert!(find_in_buffer(&buffer, "Asia/Tokyo").is_some());

        // Globe instrument metadata bar: clean location identity, no theatrical slogan
        assert!(find_in_buffer(&buffer, "DRIVES AUTOMATION").is_none());
        assert!(find_in_buffer(&buffer, "Earth").is_some());
    }

    #[test]
    fn test_phase5_6_location_acquisition_ping() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let now = Instant::now();
        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul, TR");
        model.config.location.latitude = 41.0138;
        model.config.location.longitude = 28.9497;
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;
        model.motion = crate::tui::motion::UiMotionState::new_at(now);

        // 1. Static state before acquisition
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_static = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer_static, "Istanbul, TR").is_some());

        // 2. Acquisition ping triggered upon accepting a new location
        model.motion.trigger(
            crate::tui::motion::TransientKind::LocationAcquisition {
                lon: 28.9497,
                lat: 41.0138,
            },
            now,
            Duration::from_millis(650),
        );
        assert!(model.motion.location_acquisition_phase(now).is_some());

        // 3. Render during active acquisition (draws expanding concentric circles)
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

        // 4. Settle after 650ms
        model.motion.tick(now + Duration::from_millis(700));
        assert!(model
            .motion
            .location_acquisition_phase(now + Duration::from_millis(700))
            .is_none());
    }

    #[test]
    fn test_phase9_2_visual_aspect_regression_across_viewports() {
        let test_viewports = [
            (120, 20), // Wide
            (100, 30), // Comfortable wide
            (80, 24),  // Standard terminal
            (65, 18),  // Compact stacked
            (50, 30),  // Narrow tall
            (60, 45),  // Very tall
        ];

        for (w, h) in test_viewports {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.location.city = String::from("Istanbul, TR");
            model.config.location.latitude = 41.01384;
            model.config.location.longitude = 28.94966;
            model.config.location.timezone = String::from("Europe/Istanbul");
            model.form.refresh_from_config(&model.config);
            model.active_tab = Tab::Location;

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok(), "Location failed to render at {w}x{h}");

            let buffer = terminal.backend().buffer().clone();
            assert!(find_in_buffer(&buffer, "Location").is_some());

            // The globe panel appears whenever the layout can give it room.
            let full_logo = w >= 76 && h >= 32;
            let chrome_rows: u16 = if full_logo { 8 } else { 1 } + 2 + 1;
            let body_h = h.saturating_sub(chrome_rows);
            let body_h = if body_h > 12 { body_h - 1 } else { body_h };
            let wide = w.saturating_sub(2) >= 80 && body_h >= 12;
            if wide || body_h.saturating_sub(6) >= 8 {
                assert!(
                    find_in_buffer(&buffer, "Earth").is_some(),
                    "Earth missing at {w}x{h}"
                );
            }
        }
    }

    #[test]
    fn test_phase9_2_acquisition_and_reticle_sharing_fitted_canvas() {
        let mut model = Model::new();
        model.config.location.city = String::from("Tokyo, JP");
        model.config.location.latitude = 35.6762;
        model.config.location.longitude = 139.6503;
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;
        model.motion.level = crate::tui::motion::MotionLevel::Instrument;

        // Trigger an active location acquisition transient
        model.motion.trigger(
            crate::tui::motion::TransientKind::LocationAcquisition {
                lon: 139.6503,
                lat: 35.6762,
            },
            std::time::Instant::now(),
            std::time::Duration::from_millis(500),
        );

        for (w, h) in [(120, 25), (85, 26), (75, 40)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok(), "Acquisition ping failed to render at {w}x{h}");
        }
    }

    #[test]
    fn test_phase9_2_resize_stability_and_cell_metrics_update() {
        let mut model = Model::new();
        model.config.location.city = String::from("Sydney, AU");
        model.config.location.latitude = -33.8688;
        model.config.location.longitude = 151.2093;
        model.config.location.timezone = String::from("Australia/Sydney");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;

        assert!(model.cell_metrics.cell_aspect > 0.0);

        // Simulate resize event
        crate::tui::update::update(&mut model, crate::tui::update::Message::Resize(140, 45));

        // Assert location data is never mutated by a resize
        assert_eq!(model.config.location.city, "Sydney, AU");
        assert!((model.config.location.latitude - (-33.8688)).abs() < 1e-6);
        assert!((model.config.location.longitude - 151.2093).abs() < 1e-6);
        assert_eq!(model.config.location.timezone, "Australia/Sydney");
        assert!(model.cell_metrics.cell_aspect > 0.0);
    }

    #[test]
    fn test_phase9_4_location_renderer_uses_location_centric_globe() {
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.config.location.city = String::from("Istanbul");
        model.config.location.latitude = 41.0082;
        model.config.location.longitude = 28.9784;

        let mut wide_terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        wide_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = wide_terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "Earth").is_some());
        assert!(find_in_buffer(&buffer, "41.00820° N").is_some());
        assert!(find_in_buffer(&buffer, "28.97840° E").is_some());

        // Compact Location keeps the factual fields but does not attempt to
        // squeeze an illegible globe into a minimal terminal.
        let mut compact_terminal = Terminal::new(TestBackend::new(50, 14)).unwrap();
        compact_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let compact_buffer = compact_terminal.backend().buffer().clone();
        assert!(find_in_buffer(&compact_buffer, "Latitude").is_some());
        assert!(find_in_buffer(&compact_buffer, "Timezone").is_some());
    }

    #[test]
    fn test_location_globe_renders_filled_land_and_a_centered_marker() {
        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul, TR");
        model.config.location.latitude = 41.0138;
        model.config.location.longitude = 28.9497;
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;
        model.motion.level = crate::config::MotionLevel::Off;

        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let (earth_x, earth_y, _) = find_in_buffer(&buffer, "╭ Earth").expect("earth panel");

        let braille = (earth_y..buffer.area.height)
            .flat_map(|y| (earth_x..buffer.area.width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                buffer
                    .get(x, y)
                    .symbol()
                    .chars()
                    .next()
                    .is_some_and(|c| ('\u{2801}'..='\u{28FF}').contains(&c))
            })
            .count();
        assert!(
            braille > 300,
            "globe should be filled, found {braille} braille cells"
        );

        // The configured location is the projection centre, so its marker sits
        // near the middle of the Earth panel.
        let marker = (earth_y + 1..buffer.area.height - 2)
            .flat_map(|y| (earth_x..buffer.area.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let cell = buffer.get(x, y);
                cell.symbol() == "●" && cell.fg == model.config.tui.theme.palette().accent
            })
            .expect("location marker");
        let panel_center_x = earth_x + (buffer.area.width - earth_x) / 2;
        assert!(
            marker.0.abs_diff(panel_center_x) <= 3,
            "marker {marker:?} vs {panel_center_x}"
        );
    }
}
