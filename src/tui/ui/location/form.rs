use ratatui::{
    layout::Rect,
    text::Span,
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::tui::{theme::SemanticStyles, InputMode, Model};

use super::{format_latitude_hemisphere, format_longitude_hemisphere, LABEL_WIDTH};
use crate::tui::ui::kit::{self, FieldKind, FieldState};
use crate::tui::ui::truncate;

/// The form column only needs room for its longest value and a capsule.
pub(super) fn form_column_width(app: &Model) -> u16 {
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

/// `(label, display value, edit buffer)` for each of the four form rows.
fn form_fields(app: &Model) -> [(&'static str, String, String); 4] {
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
    [
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
    ]
}

pub(super) fn render_location_form(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    styles: &SemanticStyles,
) {
    let editing = matches!(app.input_mode, InputMode::Editing);
    let block = kit::panel("Location", app.workspace_focused(), styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let value_width = usize::from(inner.width).saturating_sub(2 + LABEL_WIDTH + 1 + 4);
    let fields = form_fields(app);

    let mut lines = Vec::new();
    for (index, (label, value, edit)) in fields.iter().enumerate() {
        let focused = app.active_setting == index && app.workspace_focused();
        lines.push(kit::field_line(
            label,
            LABEL_WIDTH,
            &truncate(value, value_width),
            FieldKind::Text,
            FieldState::new(focused, focused && editing),
            Some(&truncate(edit, value_width)),
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

    render_config_error(f, app, area, styles);
}

/// Shows the config error on the row just below the form, if it fits.
fn render_config_error(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if let Some(error) = &app.config_error {
        if area.y + area.height < f.size().height {
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!(
                        " ✕ {}",
                        truncate(error, usize::from(area.width.saturating_sub(4)))
                    ),
                    styles.status_error,
                )),
                Rect::new(area.x, area.y + area.height, area.width, 1),
            );
        }
    }
}
