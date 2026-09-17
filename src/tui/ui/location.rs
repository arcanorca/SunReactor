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
