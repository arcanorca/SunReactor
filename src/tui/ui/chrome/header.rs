use std::time::Instant;

use ratatui::{
    layout::{Alignment, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::kit;
use crate::tui::model::{DaemonConnection, DaemonLifecycle};
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;

// Exact masthead from the original SunReactor chrome (a535983).
pub(crate) const ORIGINAL_SUNREACTOR_LOGO: [&str; 5] = [
    r"  ____              ____                 _            ",
    r" / ___| _   _ _ __ |  _ \ ___  __ _  ___| |_ ___  _ __",
    r" \___ \| | | | '_ \| |_) / _ \/ _` |/ __| __/ _ \| '__|",
    r"  ___) | |_| | | | |  _ <  __/ (_| | (__| || (_) | |   ",
    r" |____/ \__,_|_| |_|_| \_\___|\__,_|\___|\__\___/|_|   ",
];

/// Rows used by the full masthead: artwork, a breathing row, and status.
pub(crate) const FULL_HEADER_HEIGHT: u16 = 7;

pub(crate) fn render_header(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let now = Instant::now();

    if area.height >= FULL_HEADER_HEIGHT {
        render_masthead(
            f,
            app,
            Rect::new(area.x, area.y, area.width, 5),
            styles,
            now,
        );
        let status = status_segments(app, area.width.saturating_sub(4), styles, now, true);
        f.render_widget(
            Paragraph::new(Line::from(status)).alignment(Alignment::Center),
            Rect::new(area.x, area.y + 6, area.width, 1),
        );
        return;
    }

    // Compact identity row: mark and name on the left, live context after it,
    // and the local clock anchored right.
    let time = clock_text(app);
    let time_width = kit::cell_width(&time) as u16 + 1;
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(kit::BRAND_MARK, styles.chrome_title_secondary),
        Span::raw(" "),
        Span::styled("Sun", styles.chrome_title_secondary),
        Span::styled("Reactor", brand_reactor_style(app, styles, now)),
        Span::raw("   "),
    ];
    let budget = area.width.saturating_sub(time_width + 16);
    spans.extend(status_segments(app, budget, styles, now, false));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{time} "),
            styles.text_primary.add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Right),
        Rect::new(
            area.x + area.width.saturating_sub(time_width),
            area.y,
            time_width.min(area.width),
            1,
        ),
    );
}

fn brand_reactor_style(
    app: &Model,
    styles: &SemanticStyles,
    now: Instant,
) -> ratatui::style::Style {
    if app.motion.masthead_sweep_phase(now).is_some() {
        styles.data_value
    } else {
        styles.chrome_title
    }
}

/// The dual-tone original wordmark. Full effects reveal it once with a short
/// left-to-right power-on sweep; Reduced and Off draw it immediately.
fn render_masthead(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles, now: Instant) {
    let sweep = app.motion.masthead_sweep_phase(now);
    let art_width = ORIGINAL_SUNREACTOR_LOGO
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let lines = ORIGINAL_SUNREACTOR_LOGO
        .iter()
        .enumerate()
        .map(|(row, line)| {
            let tone = if row == 2 {
                styles.chrome_title_secondary
            } else {
                styles.chrome_title
            };
            match sweep {
                None => Line::from(Span::styled(*line, tone)),
                Some(phase) => {
                    let head = phase * (art_width as f32 + 6.0) - 3.0;
                    Line::from(
                        line.chars()
                            .enumerate()
                            .map(|(column, character)| {
                                let distance = column as f32 - head;
                                let style = if distance > 0.5 {
                                    styles.border_normal
                                } else if distance > -2.5 {
                                    styles.data_value
                                } else {
                                    tone
                                };
                                Span::styled(character.to_string(), style)
                            })
                            .collect::<Vec<_>>(),
                    )
                }
            }
        })
        .collect::<Vec<_>>();
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn clock_text(app: &Model) -> String {
    let now_local = app.current_local_time();
    if app.config.tui.use_12h_time {
        now_local.format("%I:%M:%S %p").to_string()
    } else {
        now_local.format("%H:%M:%S").to_string()
    }
}

/// The daemon badge plus live context, shortened to fit `width` cells.
fn status_segments(
    app: &Model,
    width: u16,
    styles: &SemanticStyles,
    now: Instant,
    include_clock: bool,
) -> Vec<Span<'static>> {
    let pulse = app.motion.heartbeat_pulse_phase(now).is_some();
    let badge = match app.daemon_lifecycle() {
        DaemonLifecycle::Active => vec![
            Span::styled(
                format!("{} ", kit::DOT),
                if pulse {
                    styles.data_value
                } else {
                    styles.status_success
                },
            ),
            Span::styled("Live", styles.status_success),
        ],
        DaemonLifecycle::Suspended => vec![Span::styled(
            format!("{} Suspended", kit::WARN),
            styles.status_warning.add_modifier(Modifier::BOLD),
        )],
        DaemonLifecycle::IdleDimmed => vec![Span::styled(
            format!("{} Idle dimmed", kit::WARN),
            styles.status_warning.add_modifier(Modifier::BOLD),
        )],
        DaemonLifecycle::Unreachable if app.daemon_connection == DaemonConnection::Unknown => {
            vec![Span::styled(
                format!("{} Connecting", kit::HOLLOW),
                styles.text_muted,
            )]
        }
        DaemonLifecycle::Unreachable => vec![Span::styled(
            format!("{} Offline", kit::HOLLOW),
            styles.status_error.add_modifier(Modifier::BOLD),
        )],
    };

    // Only what is useful at a glance: what each display is at right now,
    // the temperature outside, and the time.
    let mut optional: Vec<Vec<Span<'static>>> = Vec::new();
    if let Some(status) = app.status.as_ref() {
        let names: Vec<String> = status
            .monitors
            .iter()
            .map(|monitor| super::super::monitors::display_name(app, &monitor.logical_id))
            .collect();
        let short: Vec<String> = names
            .iter()
            .map(|name| name.split_whitespace().next().unwrap_or(name).to_owned())
            .collect();
        let unique = short
            .iter()
            .enumerate()
            .all(|(index, name)| !short[..index].contains(name));
        for (index, monitor) in status.monitors.iter().enumerate() {
            let label = if unique { &short[index] } else { &names[index] };
            let value = super::super::monitors::valid_applied_percent(monitor.last_applied_percent)
                .map_or_else(|| String::from("—"), |percent| format!("{percent}%"));
            optional.push(vec![
                Span::styled(format!("{label} "), styles.text_muted),
                Span::styled(value, styles.text_primary.add_modifier(Modifier::BOLD)),
            ]);
        }
        if let Some(temperature) = status
            .weather
            .as_ref()
            .and_then(|weather| weather.temperature)
        {
            optional.push(vec![Span::styled(
                super::super::weather_model::format_temp(
                    temperature,
                    app.config.tui.temperature_unit,
                ),
                styles.text_primary,
            )]);
        }
    }
    if include_clock {
        optional.push(vec![Span::styled(
            clock_text(app),
            styles.text_primary.add_modifier(Modifier::BOLD),
        )]);
    }

    let mut spans = badge;
    let mut used: usize = spans.iter().map(Span::width).sum();
    for segment in optional {
        let segment_width: usize =
            segment.iter().map(Span::width).sum::<usize>() + kit::SEPARATOR.len();
        if used + segment_width > usize::from(width) {
            continue;
        }
        used += segment_width;
        spans.push(Span::styled(kit::SEPARATOR, styles.border_normal));
        spans.extend(segment);
    }
    spans
}
