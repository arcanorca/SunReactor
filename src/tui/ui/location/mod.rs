//! The Location workspace (Tab 3): form, solar facts, globe and city picker.

mod earth;
mod form;
mod globe;
mod popup;
mod sun;
#[cfg(test)]
mod tests;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::tui::{theme::SemanticStyles, Model};

use earth::render_earth_panel;
use form::{form_column_width, render_location_form};
use popup::render_autocomplete_popup;
use sun::render_sun_panel;

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
        render_wide(f, app, area, styles);
    } else {
        render_narrow(f, app, area, styles);
    }
}

/// Two columns: form and sun facts on the left, the globe on the right.
fn render_wide(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
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
}

/// Single column: the form, then the globe or (when short) the sun facts.
fn render_narrow(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
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
