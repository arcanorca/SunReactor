use std::time::Instant;

use ratatui::{
    layout::{Alignment, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::kit;
use crate::tui::command::UiCommandContext;
use crate::tui::model::{ActionState, DaemonConnection, DaemonLifecycle, ErrorCategory};
use crate::tui::theme::{Palette, SemanticStyles};
use crate::tui::{Model, MotionLevel, Tab};

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
            .map(|monitor| super::monitors::display_name(app, &monitor.logical_id))
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
            let value = super::monitors::valid_applied_percent(monitor.last_applied_percent)
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
                super::weather_model::format_temp(temperature, app.config.tui.temperature_unit),
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

const TAB_GAP: u16 = 3;

fn tab_label(tab: Tab, compact: bool) -> String {
    if compact {
        let short = match tab {
            Tab::Monitors => "Mon",
            Tab::Limits => "Auto",
            Tab::Location => "Loc",
            Tab::Weather => "Wx",
            Tab::Settings => "Set",
        };
        format!("{} {short}", tab.index() + 1)
    } else {
        format!("{} {}", tab.index() + 1, tab.title())
    }
}

/// Tabs sit on a hairline rule; a heavier accent segment under the active tab
/// makes the current workspace visible without brackets or colour alone.
pub(crate) fn render_tabs(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let compact = area.width < 72;
    let labels = Tab::ALL
        .iter()
        .map(|tab| tab_label(*tab, compact))
        .collect::<Vec<_>>();
    let mut positions = Vec::with_capacity(labels.len());
    let mut x = 1u16;
    // Focus on the tab bar shows the shared cursor and a capsule on the
    // active workspace, exactly like a focused field.
    let mut spans = vec![if app.tabs_focused {
        Span::styled("❯", styles.focus_marker)
    } else {
        Span::raw(" ")
    }];
    for (index, (tab, label)) in Tab::ALL.iter().zip(&labels).enumerate() {
        if index > 0 {
            spans.push(Span::raw(" ".repeat(usize::from(TAB_GAP))));
            x += TAB_GAP;
        }
        let width = kit::cell_width(label) as u16;
        positions.push((x, width));
        x += width;
        if *tab == app.active_tab {
            spans.push(Span::styled(
                label.clone(),
                if app.tabs_focused {
                    styles.value_capsule_focused
                } else {
                    styles.tab_active
                },
            ));
        } else {
            spans.push(Span::styled(label.clone(), styles.tab_inactive));
        }
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    if area.height < 2 {
        return;
    }
    let active = app.active_tab.index();
    let (start, width) = match app.motion.tab_slide_position(Instant::now()) {
        Some(position) => {
            let lower = position.floor().clamp(0.0, 4.0) as usize;
            let upper = (lower + 1).min(positions.len() - 1);
            let t = position - lower as f32;
            let lerp =
                |a: u16, b: u16| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u16;
            (
                lerp(positions[lower].0, positions[upper].0),
                lerp(positions[lower].1, positions[upper].1),
            )
        }
        None => positions[active],
    };
    let rule_width = usize::from(area.width);
    let start = usize::from(start).min(rule_width);
    let end = (start + usize::from(width)).min(rule_width);
    let rule = Line::from(vec![
        Span::styled("─".repeat(start), styles.border_normal),
        Span::styled("━".repeat(end - start), styles.focus_marker),
        Span::styled("─".repeat(rule_width - end), styles.border_normal),
    ]);
    f.render_widget(
        Paragraph::new(rule),
        Rect::new(area.x, area.y + 1, area.width, 1),
    );
}

fn command_spans(
    commands: &[(&'static str, &'static str)],
    styles: &SemanticStyles,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (index, (key, label)) in commands.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(*key, styles.key_hint));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, styles.key_hint_desc));
    }
    spans
}

fn truncate_message(message: &str, width: usize) -> String {
    super::truncate(message, width.saturating_sub(4))
}

pub(crate) fn render_footer(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let row = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
    let width = usize::from(area.width);
    let now = Instant::now();
    let animated = app.motion.level == MotionLevel::Instrument;

    let line = match &app.action_state {
        ActionState::Pending {
            description,
            started_at,
            ..
        } => {
            let mut spans = vec![
                Span::raw(" "),
                Span::styled(
                    kit::spinner_frame(*started_at, now, app.motion.level != MotionLevel::Off),
                    styles.focus_marker,
                ),
                Span::raw(" "),
            ];
            spans.extend(kit::shimmer_spans(
                &truncate_message(description, width),
                *started_at,
                now,
                styles.text_muted,
                styles.text_primary.add_modifier(Modifier::BOLD),
                animated,
            ));
            Line::from(spans)
        }
        ActionState::Success { message, .. } => Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("✓ {}", truncate_message(message, width)),
                styles.status_success.add_modifier(Modifier::BOLD),
            ),
        ]),
        ActionState::Warning { message, .. } => Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("{} {}", kit::WARN, truncate_message(message, width)),
                styles.status_warning.add_modifier(Modifier::BOLD),
            ),
        ]),
        ActionState::Error {
            category, message, ..
        } => {
            let already_prefixed = [
                "Invalid value:",
                "Save failed:",
                "Offline:",
                "Rejected:",
                "Timeout:",
            ]
            .iter()
            .any(|prefix| message.starts_with(prefix));
            let text = if already_prefixed {
                message.clone()
            } else {
                let prefix = match category {
                    ErrorCategory::Validation => "Invalid value: ",
                    ErrorCategory::ConfigWrite => "Save failed: ",
                    ErrorCategory::DaemonUnavailable => "Offline: ",
                    ErrorCategory::DaemonRejected => "Rejected: ",
                    ErrorCategory::Timeout => "Timeout: ",
                    ErrorCategory::Transport => "IPC error: ",
                };
                format!("{prefix}{message}")
            };
            Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("✕ {}", truncate_message(&text, width)),
                    styles.status_error.add_modifier(Modifier::BOLD),
                ),
            ])
        }
        ActionState::Idle => idle_footer_line(app, width, styles),
    };

    f.render_widget(Paragraph::new(line).style(styles.container_bg), row);
}

/// Keys for the focused context. Global keys are dropped first, then compact
/// key glyphs are used, so advertised commands are never clipped mid-word.
fn idle_footer_line(app: &Model, width: usize, styles: &SemanticStyles) -> Line<'static> {
    let context = crate::tui::command::commands_for_footer(app);
    let weather_action = if matches!(context.context, UiCommandContext::WeatherObservational) {
        super::weather_model::weather_action_label(app)
    } else {
        None
    };
    let editing = matches!(
        context.context,
        UiCommandContext::MonitorsEdit
            | UiCommandContext::AutomationEdit
            | UiCommandContext::LocationCityEdit
            | UiCommandContext::LocationFieldEdit
            | UiCommandContext::SettingsEdit
    );

    let build = |compact: bool, globals: u8| {
        let mut list: Vec<(&'static str, &'static str)> = context
            .current_commands
            .iter()
            .map(|command| {
                let label = if matches!(command.id, crate::tui::CommandId::RetryWeather) {
                    weather_action.unwrap_or(command.footer_label)
                } else {
                    command.footer_label
                };
                (
                    if compact {
                        command.compact_keys
                    } else {
                        command.keys
                    },
                    label,
                )
            })
            .collect();
        if globals > 0 {
            // Level 1 keeps only Help and Quit, which must stay discoverable.
            list.extend(
                context
                    .global_commands
                    .iter()
                    .filter(|command| {
                        globals > 1
                            || matches!(
                                command.id,
                                crate::tui::CommandId::ToggleHelp | crate::tui::CommandId::Quit
                            )
                    })
                    .map(|command| {
                        (
                            if compact {
                                command.compact_keys
                            } else {
                                command.keys
                            },
                            command.footer_label,
                        )
                    }),
            );
        }
        list
    };

    let prefix: Vec<Span<'static>> = if editing {
        vec![
            Span::raw(" "),
            Span::styled(" EDIT ", styles.mode_edit),
            Span::raw("  "),
        ]
    } else {
        vec![Span::raw(" ")]
    };
    let prefix_width: usize = prefix.iter().map(Span::width).sum();

    for (compact, globals) in [(false, 2), (true, 2), (true, 1), (false, 0), (true, 0)] {
        let spans = command_spans(&build(compact, globals), styles);
        let line_width: usize = spans.iter().map(Span::width).sum();
        if prefix_width + line_width <= width {
            let mut all = prefix;
            all.extend(spans);
            return Line::from(all);
        }
    }

    // Last resort on very narrow terminals: as many compact commands as fit.
    let mut all = prefix;
    let mut used = prefix_width;
    for (key, label) in build(true, 0) {
        let item = kit::cell_width(key) + kit::cell_width(label) + 4;
        if used + item > width {
            break;
        }
        used += item;
        all.extend(command_spans(&[(key, label)], styles));
        all.push(Span::raw("   "));
    }
    Line::from(all)
}

pub(super) fn render_help(f: &mut Frame, app: &Model, palette: &Palette) {
    let screen = f.size();
    let styles = palette.styles();
    let width = screen.width.saturating_sub(4).clamp(30, 72);
    let height = screen.height.saturating_sub(2).clamp(8, 26);
    let area = kit::centered(screen, width, height);

    f.render_widget(Clear, area);

    let context = crate::tui::command::commands_for_workspace(app);
    let key_width = 16;
    let inner_width = area.width.saturating_sub(4);
    let mut lines = Vec::new();

    let command_line = |keys: &'static str, action: &'static str| {
        Line::from(vec![
            Span::styled(
                kit::pad_to(keys, key_width),
                styles.key_hint.add_modifier(Modifier::BOLD),
            ),
            Span::styled(action, styles.text_primary),
        ])
    };

    lines.push(kit::heading(context.context.title(), inner_width, &styles));
    if context.current_commands.is_empty() {
        lines.push(Line::from(Span::styled(
            "Nothing to adjust here.",
            styles.text_muted,
        )));
    } else {
        for command in context.current_commands {
            lines.push(command_line(command.keys, command.action));
        }
    }

    if !context.global_commands.is_empty() {
        lines.push(Line::from(""));
        lines.push(kit::heading("Everywhere", inner_width, &styles));
        for command in context.global_commands {
            lines.push(command_line(command.keys, command.action));
        }
    }

    lines.push(Line::from(""));
    lines.push(kit::heading("Symbols", inner_width, &styles));
    for item in crate::tui::command::OFFICIAL_SYMBOLS {
        lines.push(Line::from(vec![
            Span::styled(kit::pad_to(item.symbol, key_width), styles.focus_marker),
            Span::styled(item.meaning, styles.text_muted),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Effects  ", styles.data_label),
        Span::styled(app.config.tui.effects.label(), styles.text_primary),
    ]));

    let inner_height = area.height.saturating_sub(2);
    let max_scroll = lines.len().saturating_sub(usize::from(inner_height));
    let scroll = app.help_scroll.min(max_scroll);
    let title = if max_scroll > 0 {
        format!("Help · {}/{}", scroll + 1, max_scroll + 1)
    } else {
        String::from("Help")
    };
    let block = kit::with_meta(
        kit::panel(&title, true, &styles),
        vec![
            Span::styled("Esc", styles.key_hint),
            Span::styled(" close", styles.key_hint_desc),
        ],
    );

    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll as u16, 0))
            .style(styles.base)
            .block(block),
        area,
    );
}
