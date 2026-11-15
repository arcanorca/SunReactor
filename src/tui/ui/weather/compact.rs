use ratatui::{
    layout::{Alignment, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use super::super::kit;
use super::super::weather_model::WeatherViewModel;
use super::conditions::freshness_spans;
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;

pub(super) fn render_tiny(
    f: &mut Frame,
    _app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            view.temperature_label.clone(),
            styles.text_heading.add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            view.condition_label.clone(),
            styles.text_primary.add_modifier(Modifier::BOLD),
        ),
    ])];
    if let Some(cloud) = view.cloud_cover_percent {
        lines.push(Line::from(vec![
            Span::styled("Cloud cover ", styles.data_label),
            Span::styled(format!("{cloud}%"), styles.text_primary),
        ]));
    }
    lines.push(Line::from(Span::styled(
        view.location_label.clone(),
        styles.text_muted,
    )));
    lines.push(Line::from(freshness_spans(view, styles)));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}

pub(super) fn render_status_only(
    f: &mut Frame,
    _app: &Model,
    area: Rect,
    view: &WeatherViewModel,
    styles: &SemanticStyles,
) {
    let width = area.width.min(64);
    let height = area.height.min(7);
    let panel_area = kit::centered(area, width, height);
    let block = kit::panel("Weather", false, styles);
    let lines = vec![
        Line::from(""),
        Line::from(freshness_spans(
            &WeatherViewModel {
                status_detail: String::new(),
                ..view.clone()
            },
            styles,
        )),
        Line::from(Span::styled(view.status_detail.clone(), styles.text_muted)),
    ];
    f.render_widget(Clear, panel_area);
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(styles.base)
            .block(block),
        panel_area,
    );
}
