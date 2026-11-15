use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::weather_model::weather_view_model;
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;

mod compact;
mod conditions;
mod details;
mod forecast;
mod strip;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeatherResponsiveMode {
    Large,
    Medium,
    Tiny,
}

impl WeatherResponsiveMode {
    fn from_rect(area: Rect) -> Self {
        if area.width >= 96 && area.height >= 18 {
            Self::Large
        } else if area.width >= 48 && area.height >= 13 {
            Self::Medium
        } else {
            Self::Tiny
        }
    }
}

/// Weather mirrors a small weather station: what the sky is doing now (left),
/// the next day (middle), what it means for SunReactor and a few secondary
/// readings (right), and the sun and location underneath. Each value appears
/// in one place only.
pub(crate) fn render_weather(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let view = weather_view_model(app);
    if !view.has_current_conditions {
        compact::render_status_only(f, app, area, &view, styles);
        return;
    }
    if WeatherResponsiveMode::from_rect(area) == WeatherResponsiveMode::Tiny {
        compact::render_tiny(f, app, area, &view, styles);
        return;
    }

    let strip_height = if area.height >= 26 && area.width >= 90 {
        4
    } else {
        0
    };
    let body = Rect::new(
        area.x,
        area.y,
        area.width,
        area.height.saturating_sub(strip_height),
    );
    let left_width = (area.width.saturating_mul(24) / 100).clamp(32, 40);
    let right_width = (area.width.saturating_mul(24) / 100).clamp(34, 42);
    let three_columns = area.width >= left_width + right_width + 46;

    if area.width < 72 {
        conditions::render_conditions(f, app, body, &view, styles);
    } else if three_columns {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_width),
                Constraint::Length(1),
                Constraint::Min(40),
                Constraint::Length(1),
                Constraint::Length(right_width),
            ])
            .split(body);
        conditions::render_conditions(f, app, columns[0], &view, styles);
        forecast::render_forecast(f, app, columns[2], &view, styles);
        let impact_height = 6.min(columns[4].height);
        details::render_impact(
            f,
            Rect::new(columns[4].x, columns[4].y, columns[4].width, impact_height),
            &view,
            styles,
        );
        if columns[4].height > impact_height + 4 {
            details::render_additional(
                f,
                app,
                Rect::new(
                    columns[4].x,
                    columns[4].y + impact_height + 1,
                    columns[4].width,
                    columns[4].height - impact_height - 1,
                ),
                &view,
                styles,
            );
        }
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_width),
                Constraint::Length(1),
                Constraint::Min(36),
            ])
            .split(body);
        conditions::render_conditions(f, app, columns[0], &view, styles);
        forecast::render_forecast(f, app, columns[2], &view, styles);
    }
    if strip_height > 0 {
        strip::render_sun_strip(
            f,
            app,
            Rect::new(area.x, area.y + body.height, area.width, strip_height),
            &view,
            styles,
        );
    }
}

#[cfg(test)]
mod tests {
    include!("tests.rs");
}
