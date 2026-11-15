use std::time::Instant;

use chrono::Utc;
use ratatui::{
    layout::{Alignment, Rect},
    style::Modifier,
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Circle, Points},
        Clear, Paragraph,
    },
    Frame,
};

use crate::tui::globe::{
    center_for_motion, daylight_at, fit_globe_viewport, project_orthographic, Daylight, GlobeCenter,
};
use crate::tui::ui::kit;
use crate::tui::{theme::SemanticStyles, Model};

use super::globe::{shade_globe, GlobeLayers};

/// Projected overlay positions (and pulse phase) for one globe frame.
struct GlobeMarks {
    marker: Option<(f64, f64)>,
    sun: Option<(f64, f64)>,
    pulse: Option<f32>,
}

pub(super) fn render_earth_panel(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
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
    let marks = globe_marks(app, target, subsolar, center, now);
    render_globe(f, viewport, &layers, &marks, styles);

    if legend_rows > 0 {
        render_legend(f, app, inner, styles);
    }
}

fn globe_marks(
    app: &Model,
    target: GlobeCenter,
    subsolar: (f64, f64),
    center: GlobeCenter,
    now: Instant,
) -> GlobeMarks {
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
    GlobeMarks { marker, sun, pulse }
}

fn render_globe(
    f: &mut Frame,
    viewport: Rect,
    layers: &GlobeLayers,
    marks: &GlobeMarks,
    styles: &SemanticStyles,
) {
    let GlobeMarks { marker, sun, pulse } = *marks;
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
}

fn render_legend(f: &mut Frame, app: &Model, inner: Rect, styles: &SemanticStyles) {
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
        Paragraph::new(legend).alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
    );
}
