use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
    Frame,
};

use super::super::weather_model::WeatherViewModel;
use super::super::{automation, kit};
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;

/// Sun times, daylight, location, and data source along the bottom.
pub(super) fn render_sun_strip(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles.border_normal)
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.height < 2 {
        return;
    }
    let (mut items, place) = sun_strip_items(app, view);
    fit_sun_strip_items(&mut items, &place, inner.width);
    render_sun_strip_items(f, inner, &items, styles);
}

struct SunStripItem {
    glyph: &'static str,
    label: &'static str,
    value: String,
}

fn sun_strip_items(app: &Model, view: &WeatherViewModel) -> (Vec<SunStripItem>, String) {
    let anchors = automation::cycle_anchors(app, None);
    let time = |value: Option<&chrono::DateTime<chrono::FixedOffset>>| {
        value.map_or_else(
            || String::from("—"),
            |time| automation::format_time(time, app.config.tui.use_12h_time),
        )
    };
    let daylight = match (&anchors.sunrise, &anchors.sunset) {
        (Some(sunrise), Some(sunset)) if sunset > sunrise => {
            let minutes = (*sunset - *sunrise).num_minutes();
            format!("{}h {:02}m", minutes / 60, minutes % 60)
        }
        _ => String::from("—"),
    };
    let location = &app.config.location;
    let place = if location.city.trim().is_empty() {
        String::from("Configured location")
    } else {
        location.city.trim().to_owned()
    };
    let coordinates = format!(
        "{:.2}° {} · {:.2}° {}",
        location.latitude.abs(),
        if location.latitude < 0.0 { "S" } else { "N" },
        location.longitude.abs(),
        if location.longitude < 0.0 { "W" } else { "E" },
    );
    (
        vec![
            SunStripItem {
                glyph: "↑",
                label: "Sunrise",
                value: time(anchors.sunrise.as_ref()),
            },
            SunStripItem {
                glyph: kit::BRAND_MARK,
                label: "Solar noon",
                value: time(anchors.noon.as_ref()),
            },
            SunStripItem {
                glyph: "↓",
                label: "Sunset",
                value: time(anchors.sunset.as_ref()),
            },
            SunStripItem {
                glyph: "◐",
                label: "Daylight",
                value: daylight,
            },
            SunStripItem {
                glyph: "◎",
                label: "Location",
                value: format!("{place}  {coordinates}"),
            },
            SunStripItem {
                glyph: "≡",
                label: "Weather data",
                value: view.source_label.clone(),
            },
        ],
        place,
    )
}

fn sun_strip_width(items: &[SunStripItem]) -> usize {
    items
        .iter()
        .map(|item| 2 + kit::cell_width(item.label).max(kit::cell_width(&item.value)))
        .sum::<usize>()
        + items.len().saturating_sub(1) * 3
}

fn fit_sun_strip_items(items: &mut Vec<SunStripItem>, place: &str, width: u16) {
    for label in ["Weather data", "Daylight", "Solar noon"] {
        if sun_strip_width(items) <= usize::from(width) {
            break;
        }
        items.retain(|item| item.label != label);
    }
    if sun_strip_width(items) > usize::from(width) {
        if let Some(item) = items.iter_mut().find(|item| item.label == "Location") {
            place.clone_into(&mut item.value);
        }
    }
}

fn render_sun_strip_items(
    f: &mut Frame,
    inner: Rect,
    items: &[SunStripItem],
    styles: &SemanticStyles,
) {
    let widths: Vec<u16> = items
        .iter()
        .map(|item| (2 + kit::cell_width(item.label).max(kit::cell_width(&item.value))) as u16)
        .collect();
    let content: u16 = widths.iter().sum();
    let spacing =
        inner.width.saturating_sub(content) / (items.len().saturating_sub(1) as u16).max(1);
    let mut x = inner.x;
    for (index, (item, width)) in items.iter().zip(&widths).enumerate() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(format!("{} ", item.glyph), styles.focus_marker),
                    Span::styled(item.label, styles.text_muted),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(item.value.clone(), styles.text_primary),
                ]),
            ]),
            Rect::new(x, inner.y, (*width).min(inner.x + inner.width - x), 2),
        );
        x += width;
        if index + 1 < items.len() {
            render_sun_strip_separator(f, x + spacing / 2, inner.y, inner.x + inner.width, styles);
            x += spacing;
        }
    }
}

fn render_sun_strip_separator(f: &mut Frame, x: u16, y: u16, right: u16, styles: &SemanticStyles) {
    if x < right {
        for row in 0..2 {
            f.buffer_mut()
                .get_mut(x, y + row)
                .set_symbol("│")
                .set_style(styles.border_normal);
        }
    }
}
