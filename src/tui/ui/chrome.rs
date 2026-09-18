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

#[cfg(test)]
mod tests {

    use crate::tui::InputMode;
    use std::time::Duration;
    use std::time::Instant;

    use ratatui::{backend::TestBackend, Terminal};

    use crate::tui::model::{
        ActionKind, ActionState, AutomationRegionFocus, DaemonConnection, ErrorCategory,
    };
    use crate::tui::test_support::{
        buffer_row, buffer_text, configure_monitor_fixture, dummy_status, find_in_buffer,
        named_monitor_config, two_monitor_model,
    };
    use crate::tui::ui;
    use crate::tui::{Model, Tab};
    #[test]
    fn test_chrome_render_regression_guard() {
        use ratatui::style::Modifier;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(2));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.config.tui.theme = crate::config::Theme::Amber;
        model.config.location.city = String::from("Istanbul");
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.motion.masthead_sweep_done = true;

        let palette = model.config.tui.theme.palette();

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // 1. Compact identity keeps the dual-tone Sun/Reactor treatment.
        let (_, _, title_cell) = find_in_buffer(&buffer, "Reactor").expect("header title");
        assert_eq!(title_cell.fg, palette.accent);
        assert!(title_cell.modifier.contains(Modifier::BOLD));
        let (_, _, sun_cell) = find_in_buffer(&buffer, "Sun").expect("Sun prefix rendered");
        assert_eq!(sun_cell.fg, palette.secondary_accent);

        // 2. Live state is a glyph plus a word in the success colour, not a filled badge.
        let (x, y, live_cell) = find_in_buffer(&buffer, "Live").expect("live state");
        assert_eq!(live_cell.fg, palette.success);
        assert_ne!(live_cell.bg, palette.success);
        assert_eq!(buffer.get(x - 2, y).symbol(), "●");

        // 3. Only practical context: applied values, no sun angle, mode, or city.
        assert!(find_in_buffer(&buffer, "25%").is_some());
        assert!(find_in_buffer(&buffer, "Sun +15.0°").is_none());
        assert!(find_in_buffer(&buffer, "MODE:").is_none());
        assert!(find_in_buffer(&buffer, "Istanbul").is_none());

        // 5. Active tab: accent, bold, and an accent rule segment directly below.
        let (tab_x, tab_y, active_tab_cell) =
            find_in_buffer(&buffer, "1 Monitors").expect("active tab");
        assert_eq!(active_tab_cell.fg, palette.accent);
        assert_ne!(active_tab_cell.bg, palette.accent);
        assert!(active_tab_cell.modifier.contains(Modifier::BOLD));
        let indicator = buffer.get(tab_x, tab_y + 1);
        assert_eq!(indicator.symbol(), "━");
        assert_eq!(indicator.fg, palette.accent);
        assert!(find_in_buffer(&buffer, "[1 MONITORS]").is_none());

        // 6. Inactive tabs are muted.
        let (_, _, inactive_tab_cell) = find_in_buffer(&buffer, "2 Automation").expect("tab");
        assert_ne!(inactive_tab_cell.fg, palette.accent);

        // 7. Footer: muted key descriptions on the last row, no mode chip in navigation.
        let (_, hint_y, hint_cell) = find_in_buffer(&buffer, "Select").expect("footer hint");
        assert_eq!(hint_cell.fg, palette.text_muted);
        assert_eq!(hint_y, 23);
        assert!(find_in_buffer(&buffer, "NAV").is_none());

        // 8. Warning feedback replaces the footer with a warning-coloured message.
        model.action_state = ActionState::Warning {
            action: ActionKind::SaveConfig,
            category: ErrorCategory::DaemonUnavailable,
            message: String::from("Saved to disk — daemon offline"),
            completed_at: Instant::now(),
        };
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let warn_buffer = terminal.backend().buffer().clone();
        let (_, warn_y, warn_cell) =
            find_in_buffer(&warn_buffer, "Saved to disk — daemon offline").expect("warning");
        assert_eq!(warn_cell.fg, palette.warning);
        assert!(warn_cell.modifier.contains(Modifier::BOLD));
        assert_eq!(warn_y, 23);

        // 9. Error feedback uses the error colour and a category prefix.
        model.action_state = ActionState::Error {
            action: ActionKind::SaveConfig,
            category: ErrorCategory::ConfigWrite,
            message: String::from("Permission denied"),
            completed_at: Instant::now(),
        };
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let err_buffer = terminal.backend().buffer().clone();
        let (_, _, err_cell) =
            find_in_buffer(&err_buffer, "Save failed: Permission denied").expect("error");
        assert_eq!(err_cell.fg, palette.error);
        assert!(err_cell.modifier.contains(Modifier::BOLD));

        // 10. Offline daemon state.
        model.action_state = ActionState::Idle;
        model.daemon_connection = DaemonConnection::Disconnected;
        model.status = None;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let offline_buffer = terminal.backend().buffer().clone();
        let (_, _, offline_cell) = find_in_buffer(&offline_buffer, "Offline").expect("offline");
        assert_eq!(offline_cell.fg, palette.error);
        assert!(offline_cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_semantic_styles_chrome_slice_direct_render() {
        use ratatui::layout::Rect;
        use ratatui::style::{Color, Modifier, Style};

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.motion.masthead_sweep_done = true;

        // Create custom semantic styles with distinguishable overrides
        let base_palette = crate::config::Theme::Amber.palette();
        let mut custom_styles = base_palette.styles();
        custom_styles.chrome_title = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::UNDERLINED);
        custom_styles.tab_active = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        custom_styles.key_hint_desc = Style::default().fg(Color::LightGreen);

        terminal
            .draw(|f| {
                // Render individual chrome components directly using the semantic styles slice
                super::render_header(f, &model, Rect::new(0, 0, 80, 3), &custom_styles);
                super::render_tabs(f, &model, Rect::new(0, 3, 80, 3), &custom_styles);
                super::render_footer(f, &model, Rect::new(0, 21, 80, 3), &custom_styles);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // 1. Direct header render consumed custom_styles.chrome_title on Reactor.
        let title_match = find_in_buffer(&buffer, "Reactor");
        assert!(title_match.is_some(), "Header title not found");
        let (_, _, title_cell) = title_match.unwrap();
        assert_eq!(title_cell.fg, Color::Cyan);
        assert!(title_cell.modifier.contains(Modifier::UNDERLINED));

        // 2. Direct tabs render consumed custom_styles.tab_active
        let tab_match = find_in_buffer(&buffer, "1 Monitors");
        assert!(tab_match.is_some(), "Active tab not found");
        let (_, _, tab_cell) = tab_match.unwrap();
        assert_eq!(tab_cell.fg, Color::Yellow);
        assert!(tab_cell.modifier.contains(Modifier::BOLD));

        // 3. Direct footer render consumed custom_styles.key_hint_desc
        let footer_match = find_in_buffer(&buffer, "Select");
        assert!(footer_match.is_some(), "Footer hint not found");
        let (_, _, footer_cell) = footer_match.unwrap();
        assert_eq!(footer_cell.fg, Color::LightGreen);
    }

    #[test]
    fn test_chrome_compact_responsive_mode() {
        let backend = TestBackend::new(60, 18);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // 1. Compact header contains identity and badge
        assert!(find_in_buffer(&buffer, "SunReactor").is_some());
        assert!(find_in_buffer(&buffer, "Live").is_some());

        // 2. Compact tabs use short titles; the rule marks the active one.
        assert!(find_in_buffer(&buffer, "1 Mon").is_some());
        assert!(find_in_buffer(&buffer, "2 Auto").is_some());
        assert!(find_in_buffer(&buffer, "━").is_some());

        // 3. Compact footer keeps its commands readable.
        assert!(find_in_buffer(&buffer, "Select").is_some());
        assert!(find_in_buffer(&buffer, "Susp/Res").is_some());
    }

    #[test]
    fn test_chrome_editing_mode_footer() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.start_editing();

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Footer shows EDIT mode tag and editing-specific commands
        assert!(find_in_buffer(&buffer, "EDIT").is_some());
        assert!(find_in_buffer(&buffer, "Move").is_some());
        assert!(find_in_buffer(&buffer, "Save").is_some());
        assert!(find_in_buffer(&buffer, "Cancel").is_some());
    }

    #[test]
    fn test_chrome_automation_mode_footer() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;

        // Automation opens on the curve control.
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "Monitor").is_some());
        assert!(find_in_buffer(&buffer, "Gamma").is_some());
        assert!(find_in_buffer(&buffer, "Schedule").is_some());
        assert!(find_in_buffer(&buffer, "±1 min").is_none());

        // The schedule advertises only milestone commands.
        model.automation_focus = AutomationRegionFocus::Milestones;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "±1 min").is_some());
        assert!(find_in_buffer(&buffer, "Reset").is_some());
        assert!(find_in_buffer(&buffer, "Left/Right change monitor").is_none());
    }

    #[test]
    fn test_settings_footer_compacts_before_clipping_at_comfortable_boundary() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut model = Model::new();
        model.active_tab = Tab::Settings;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "? Help").is_some());
        assert!(find_in_buffer(&buffer, "q Quit").is_some());
    }

    #[test]
    fn test_chrome_non_amber_palettes() {
        use ratatui::style::Modifier;

        for theme in [
            crate::config::Theme::Nord,
            crate::config::Theme::TokyoNight,
            crate::config::Theme::HackerGreen,
            crate::config::Theme::Commodore64,
            crate::config::Theme::Grayscale,
        ] {
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.theme = theme;
            model.status = Some(dummy_status(1));
            model.daemon_connection = DaemonConnection::Connected;
            model.active_tab = Tab::Monitors;
            model.motion.masthead_sweep_done = true;

            let palette = theme.palette();

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            // 1. Product title renders with the primary theme accent.
            let title_match = find_in_buffer(&buffer, "Reactor");
            assert!(title_match.is_some(), "Title missing in theme {theme:?}");
            let (_, _, title_cell) = title_match.unwrap();
            assert_eq!(title_cell.fg, palette.accent);

            // 2. Active tab renders with accent fg
            let tab_match = find_in_buffer(&buffer, "1 Monitors");
            assert!(tab_match.is_some(), "Active tab missing in theme {theme:?}");
            let (_, _, tab_cell) = tab_match.unwrap();
            assert_eq!(tab_cell.fg, palette.accent);
            assert!(tab_cell.modifier.contains(Modifier::BOLD));

            // 3. Footer commands rendered
            assert!(
                find_in_buffer(&buffer, "? Help").is_some(),
                "Footer help missing in {theme:?}"
            );
        }
    }

    #[test]
    fn test_responsive_rendering_across_target_dimensions() {
        let tabs = [
            Tab::Monitors,
            Tab::Limits,
            Tab::Location,
            Tab::Weather,
            Tab::Settings,
        ];

        // Targets: 80x24 (Comfortable), 60x20 (Compact), 40x16 (Minimal)
        let sizes = [(80, 24), (60, 20), (40, 16)];

        for (w, h) in sizes {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();

            for tab in tabs {
                let mut model = Model::new();
                model.status = Some(dummy_status(3));
                model.active_tab = tab;

                terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            }
        }
    }

    #[test]
    fn test_responsive_rendering_too_small_safe_fallback() {
        // Deliberately tiny terminals: 30x8, 20x5, 10x2
        let tiny_sizes = [(30, 8), (20, 5), (10, 2)];

        for (w, h) in tiny_sizes {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.status = Some(dummy_status(2));

            // Must not panic!
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        }
    }

    #[test]
    fn test_footer_renders_action_states_across_responsive_viewports() {
        let dimensions = [(80, 24), (60, 20), (40, 16)];

        for (w, h) in dimensions {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut model = Model::new();
            model.status = Some(dummy_status(2));

            // Test Pending rendering
            model.action_state = ActionState::Pending {
                command_id: 1,
                action: ActionKind::Suspend,
                description: String::from("Suspending writes…"),
                started_at: Instant::now(),
            };
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

            // Test Success rendering
            model.action_state = ActionState::Success {
                action: ActionKind::Suspend,
                message: String::from("Writes suspended"),
                completed_at: Instant::now(),
            };
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

            // Test Warning rendering (partial success)
            model.action_state = ActionState::Warning {
                action: ActionKind::SaveConfig,
                category: ErrorCategory::DaemonUnavailable,
                message: String::from("Saved to disk — daemon offline"),
                completed_at: Instant::now(),
            };
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

            // Test Error rendering
            model.action_state = ActionState::Error {
                action: ActionKind::SaveConfig,
                category: ErrorCategory::ConfigWrite,
                message: String::from("permission denied"),
                completed_at: Instant::now(),
            };
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        }
    }

    #[test]
    fn test_compact_brand_comfortable_viewport() {
        let backend = TestBackend::new(85, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Compact dual-tone identity leaves the workspace as the primary surface.
        assert!(find_in_buffer(&buffer, "SunReactor").is_some());
        assert!(find_in_buffer(&buffer, "____").is_none());

        // Operational telemetry stays terse.
        assert!(find_in_buffer(&buffer, "Live").is_some());
        assert!(find_in_buffer(&buffer, "MODE:").is_none());
    }

    #[test]
    fn test_phase5_5_2_compact_header_spacing_and_telemetry() {
        // 1. Comfortable Mode: compact brand plus two rows of useful telemetry.
        {
            let backend = TestBackend::new(85, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.location.city = String::from("Istanbul, TR");
            model.daemon_connection = DaemonConnection::Connected;
            model.status = Some(dummy_status(1));
            model.active_tab = Tab::Monitors;

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            // Compact product identity remains visible without consuming the workspace.
            assert!(find_in_buffer(&buffer, "SunReactor").is_some());
            assert!(find_in_buffer(&buffer, "____").is_none());

            // Centered telemetry cluster present
            assert!(find_in_buffer(&buffer, "Live").is_some());
            assert!(find_in_buffer(&buffer, "25%").is_some());
            assert!(find_in_buffer(&buffer, "Istanbul, TR").is_none());

            // Redundant MODE: AUTOMATIC removed
            assert!(find_in_buffer(&buffer, "MODE:").is_none());
            assert!(find_in_buffer(&buffer, "MODE: AUTOMATIC").is_none());
        }

        // 2. Compact Mode (65x18): No redundant AUTOMATIC in header
        {
            let backend = TestBackend::new(65, 18);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.location.city = String::from("Istanbul");
            model.daemon_connection = DaemonConnection::Connected;
            model.status = Some(dummy_status(1));
            model.active_tab = Tab::Location;

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "SunReactor").is_some());
            assert!(find_in_buffer(&buffer, "Live").is_some());
            assert!(find_in_buffer(&buffer, "AUTOMATIC").is_none());
        }

        // 3. Minimal Mode (45x12): One-line header
        {
            let backend = TestBackend::new(45, 12);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.status = Some(dummy_status(1));
            model.daemon_connection = DaemonConnection::Connected;
            model.active_tab = Tab::Location;

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "SunReactor").is_some());
            assert!(find_in_buffer(&buffer, "Live").is_some());
            assert!(find_in_buffer(&buffer, "AUTOMATIC").is_none());
        }
    }

    #[test]
    fn test_phase9_4_1_full_header_reserves_a_breathing_row() {
        let mut model = Model::new();
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
        );
        model.config.tui.show_logo = true;
        model.config.location.city = String::from("Istanbul, TR");
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;

        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(buffer_row(&buffer, 0).contains("____"));
        assert!(buffer_row(&buffer, 4).contains("|____"));
        assert!(buffer_row(&buffer, 5).trim().is_empty());
        assert!(buffer_row(&buffer, 6).contains("Live"));
        assert!(buffer_row(&buffer, 6).contains("25%"));
        assert!(!buffer_row(&buffer, 6).contains("Sun +"));
        assert!(buffer_row(&buffer, 7).trim().is_empty());
        assert!(buffer_row(&buffer, 8).contains("1 Monitors"));
        assert!(buffer_row(&buffer, 9).contains('━'));

        let mut compact_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let compact = compact_terminal.draw(|f| ui::ui(f, &mut model));
        assert!(compact.is_ok());
    }

    #[test]
    fn test_footer_keeps_help_and_quit_when_space_is_short() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Monitors;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let footer = buffer_row(terminal.backend().buffer(), 23);
        assert!(footer.contains("? Help"), "{footer}");
        assert!(footer.contains("q Quit"), "{footer}");
    }

    #[test]
    fn test_phase5_6_brand_sweep_lifecycle() {
        let backend = TestBackend::new(85, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let now = Instant::now();
        let mut model = Model::new();
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.status = Some(dummy_status(1));
        model.motion = crate::tui::motion::UiMotionState::new_at(now);

        // 1. Startup phase is active
        let p0 = model.motion.masthead_sweep_phase(now);
        assert!(p0.is_some());

        // Render during sweep
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_sweep = terminal.backend().buffer().clone();

        // Compact wordmark stays intact while its primary tone sweeps.
        assert!(find_in_buffer(&buffer_sweep, "SunReactor").is_some());

        // 2. Fast forward past duration: sweep expires
        let past = now + Duration::from_millis(700);
        model.motion.tick(past);
        assert!(model.motion.masthead_sweep_done);
        assert!(model.motion.masthead_sweep_phase(past).is_none());

        // Settled render preserves the compact identity.
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_settled = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer_settled, "SunReactor").is_some());
    }

    #[test]
    fn test_phase5_6_live_heartbeat_and_offline_invariance() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let now = Instant::now();
        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.motion = crate::tui::motion::UiMotionState::new_at(now);

        let dot_cell = |buffer: &ratatui::buffer::Buffer| {
            let (x, y, _) = find_in_buffer(buffer, "● Live").expect("live state");
            buffer.get(x, y).clone()
        };

        // 0. Stable state before heartbeat
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let cell_stable = dot_cell(terminal.backend().buffer());

        // 1. A real status response pulses the live dot.
        let pulse_now = Instant::now();
        model.motion.record_heartbeat(pulse_now);
        assert!(model.motion.heartbeat_pulse_phase(pulse_now).is_some());
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let cell_pulse = dot_cell(terminal.backend().buffer());
        assert_ne!(cell_stable.fg, cell_pulse.fg);
        assert_eq!(cell_stable.bg, cell_pulse.bg);

        // 2. After the pulse the dot settles back.
        model.motion.last_heartbeat_at = Some(
            Instant::now()
                .checked_sub(Duration::from_millis(500))
                .unwrap(),
        );
        assert!(model.motion.heartbeat_pulse_phase(Instant::now()).is_none());
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let cell_settled = dot_cell(terminal.backend().buffer());
        assert_eq!(cell_settled.fg, cell_stable.fg);

        // 3. Offline is static.
        model.daemon_connection = crate::tui::model::DaemonConnection::Disconnected;
        model.status = None;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_offline = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer_offline, "○ Offline").is_some());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_phase9_context_aware_help_all_contexts() {
        let mut model = Model::new();
        model.show_help = true;

        let render = |model: &mut Model| {
            let mut terminal = Terminal::new(TestBackend::new(85, 26)).unwrap();
            terminal.draw(|f| ui::ui(f, model)).unwrap();
            terminal.backend().buffer().clone()
        };
        let assert_all = |buffer: &ratatui::buffer::Buffer, texts: &[&str]| {
            for text in texts {
                assert!(
                    find_in_buffer(buffer, text).is_some(),
                    "{text:?} missing:\n{}",
                    buffer_text(buffer)
                );
            }
        };

        // 1. Monitors list
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::List;
        model.input_mode = InputMode::Normal;
        let buffer = render(&mut model);
        assert_all(
            &buffer,
            &[
                "╭ Help",
                "Select monitor",
                "Edit brightness range",
                "Open automation for this monitor",
                "Everywhere",
                "Symbols",
            ],
        );

        // 2. Monitors range controls
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
        let buffer = render(&mut model);
        assert_all(
            &buffer,
            &[
                "Choose minimum or maximum",
                "Enter exact brightness value",
                "Adjust selected brightness ±1%",
                "Back to monitor list",
            ],
        );

        // 3. Monitors edit: global shortcuts are not advertised while typing.
        model.input_mode = InputMode::Editing;
        let buffer = render(&mut model);
        assert_all(
            &buffer,
            &["Input percentage value", "Save and apply", "Cancel editing"],
        );
        assert!(find_in_buffer(&buffer, "Everywhere").is_none());

        // 4. Automation curve (the default Automation focus)
        model.active_tab = Tab::Limits;
        model.input_mode = InputMode::Normal;
        model.automation_focus = crate::tui::model::AutomationRegionFocus::Curve;
        let buffer = render(&mut model);
        assert_all(
            &buffer,
            &[
                "Solar curvature",
                "Switch monitor",
                "Adjust gamma by 0.05",
                "Move to the schedule",
            ],
        );

        // 5. Automation schedule
        model.automation_focus = crate::tui::model::AutomationRegionFocus::Milestones;
        let buffer = render(&mut model);
        assert_all(
            &buffer,
            &[
                "Automation schedule",
                "Shift the selected milestone by 1 minute",
                "Reset the milestone to its solar time",
            ],
        );
        assert!(find_in_buffer(&buffer, "Left/Right change monitor").is_none());

        // 6. Location
        model.active_tab = Tab::Location;
        let buffer = render(&mut model);
        assert_all(&buffer, &["Select location field", "Edit selected field"]);

        // 7. City search
        model.active_setting = 0;
        model.input_mode = InputMode::Editing;
        let buffer = render(&mut model);
        assert_all(
            &buffer,
            &[
                "Search city",
                "Search worldwide cities",
                "Select autocomplete match",
                "Use city, coordinates, and timezone",
            ],
        );

        // 8. Settings
        model.active_tab = Tab::Settings;
        model.input_mode = InputMode::Normal;
        let buffer = render(&mut model);
        assert_all(&buffer, &["Select setting", "Toggle or edit setting"]);

        // 9. Settings edit
        model.active_setting = crate::tui::model::settings_index::WEATHER_API_KEY;
        model.input_mode = InputMode::Editing;
        let buffer = render(&mut model);
        assert_all(
            &buffer,
            &[
                "Edit setting",
                "Input replacement value",
                "Save configuration",
            ],
        );

        // 10. Weather without data offers no fake controls.
        model.active_tab = Tab::Weather;
        model.input_mode = InputMode::Normal;
        let buffer = render(&mut model);
        assert_all(&buffer, &["Nothing to adjust here."]);
        assert!(find_in_buffer(&buffer, "Edit operating range").is_none());
        assert!(find_in_buffer(&buffer, "Edit API key").is_none());
        assert!(find_in_buffer(&buffer, "Observational atmospheric").is_none());
    }

    #[test]
    fn test_phase9_no_stale_or_prohibited_terms() {
        let mut model = Model::new();
        model.show_help = true;
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        for tab in [
            Tab::Monitors,
            Tab::Limits,
            Tab::Location,
            Tab::Weather,
            Tab::Settings,
        ] {
            model.active_tab = tab;
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            // 1. No user-facing "Limits" string in Help
            assert!(find_in_buffer(&buffer, "Limits").is_none());
            assert!(find_in_buffer(&buffer, "LIMITS").is_none());

            // 2. No "fine-tuning" hidden-page terminology
            assert!(find_in_buffer(&buffer, "fine-tuning").is_none());
            assert!(find_in_buffer(&buffer, "fine-tune").is_none());

            // 3. No old "fine-adjust transition gamma"
            assert!(find_in_buffer(&buffer, "fine-adjust transition").is_none());
        }
    }

    #[test]
    fn test_phase9_api_key_secret_safety_in_help() {
        let mut model = Model::new();
        model.active_tab = Tab::Settings;
        model.active_setting = crate::tui::model::settings_index::WEATHER_API_KEY;
        model.input_mode = InputMode::Editing;
        model.show_help = true;

        // Put a secret string in the input
        let secret = "test_super_secret_api_key_123456789";
        for c in secret.chars() {
            model
                .form
                .api_key_input
                .handle(tui_input::InputRequest::InsertChar(c));
        }

        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Secret text MUST NEVER leak into the Help overlay
        assert!(find_in_buffer(&buffer, secret).is_none());
        assert!(find_in_buffer(&buffer, "test_super_secret").is_none());
        assert!(find_in_buffer(&buffer, "123456789").is_none());
    }

    #[test]
    fn test_phase9_symbol_legend_vocabulary() {
        let mut model = Model::new();
        model.show_help = true;
        model.active_tab = Tab::Monitors;

        let backend = TestBackend::new(85, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        for text in [
            "❯",
            "keyboard cursor",
            "›",
            "selected item in an unfocused list",
            "‹ ›",
            "value changes with ← →",
            "●",
            "current milestone",
            "○",
            "inactive or unavailable",
            "▲",
            "needs attention",
            "⎿",
            "detail for the row above",
            "Effects",
        ] {
            assert!(
                find_in_buffer(&buffer, text).is_some(),
                "{text:?} missing:\n{}",
                buffer_text(&buffer)
            );
        }
        // The legacy rail vocabulary is gone.
        assert!(find_in_buffer(&buffer, "retained context").is_none());
    }

    #[test]
    fn test_phase9_responsive_help_modal_layouts() {
        let mut model = Model::new();
        model.show_help = true;
        model.active_tab = Tab::Monitors;

        for (width, height, extra) in [(85, 26, "Symbols"), (70, 20, "Symbols"), (50, 14, "close")]
        {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(
                find_in_buffer(&buffer, "╭ Help").is_some(),
                "{width}x{height}"
            );
            assert!(find_in_buffer(&buffer, extra).is_some(), "{width}x{height}");
        }
    }

    #[test]
    fn test_phase9_theme_palettes_help_render() {
        let mut model = Model::new();
        model.show_help = true;
        model.active_tab = Tab::Monitors;

        for theme in [
            crate::tui::theme::Theme::Amber,
            crate::tui::theme::Theme::Nord,
            crate::tui::theme::Theme::TokyoNight,
            crate::tui::theme::Theme::HackerGreen,
            crate::tui::theme::Theme::Grayscale,
            crate::tui::theme::Theme::Commodore64,
        ] {
            model.config.tui.theme = theme;
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "╭ Help").is_some());
            assert!(find_in_buffer(&buffer, "Select monitor").is_some());
        }
    }

    #[test]
    fn test_phase9_1_api_key_secret_safety_comprehensive() {
        let mut model = Model::new();
        model.active_tab = Tab::Settings;
        let secret = "very_secret_api_key_never_leak_xyz";
        model.form.api_key_input = tui_input::Input::default().with_value(String::from(secret));

        // 1. Normal mode: TestBackend buffer shows fixed 8-bullet mask, never raw secret or length
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "●●●●●●●●").is_some());
        assert!(find_in_buffer(&buffer, secret).is_none());
        assert!(find_in_buffer(&buffer, "very_secret").is_none());

        // 2. Help mode: Help never receives or displays the secret
        model.show_help = true;
        let backend2 = TestBackend::new(85, 26);
        let mut terminal2 = Terminal::new(backend2).unwrap();
        terminal2.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer2 = terminal2.backend().buffer().clone();

        assert!(find_in_buffer(&buffer2, secret).is_none());
        assert!(find_in_buffer(&buffer2, "very_secret").is_none());
    }

    #[test]
    fn test_phase9_1_help_modal_scrolling_boundaries_and_resize() {
        let mut model = Model::new();
        model.show_help = true;

        // Top clamp
        model.scroll_help_up(100);
        assert_eq!(model.help_scroll, 0);

        // Page down (6 lines)
        model.scroll_help_down(6, 20);
        assert_eq!(model.help_scroll, 6);

        // Page up (6 lines)
        model.scroll_help_up(6);
        assert_eq!(model.help_scroll, 0);

        // Bottom clamp
        model.scroll_help_down(50, 15);
        assert_eq!(model.help_scroll, 15);

        // Home
        model.scroll_help_home();
        assert_eq!(model.help_scroll, 0);

        // End
        model.scroll_help_end(18);
        assert_eq!(model.help_scroll, 18);

        // Render with large scroll clamped to smaller viewport
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
    }

    #[test]
    fn test_phase9_1_responsive_help_all_contexts_no_panic() {
        let mut model = Model::new();
        model.show_help = true;

        for tab in [
            Tab::Monitors,
            Tab::Limits,
            Tab::Location,
            Tab::Weather,
            Tab::Settings,
        ] {
            model.active_tab = tab;
            for (w, h) in [(85, 26), (70, 20), (50, 14), (30, 10)] {
                let backend = TestBackend::new(w, h);
                let mut terminal = Terminal::new(backend).unwrap();
                let res = terminal.draw(|f| ui::ui(f, &mut model));
                assert!(res.is_ok(), "Failed to render Help on {tab:?} at {w}x{h}");
            }
        }
    }

    #[test]
    fn test_phase9_3_dual_tone_brand_and_sweep_lifecycle() {
        let mut model = Model::new();
        model.config.tui.effects = crate::config::MotionLevel::Instrument;
        model.motion.level = crate::config::MotionLevel::Instrument;

        let backend = TestBackend::new(85, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Initial settled render with motion: verify two-tone presence
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Compact brand retains its two-tone Sun/Reactor treatment.
        let (_, _, sun_cell) = find_in_buffer(&buffer, "Sun").expect("Sun brand segment");
        let (_, _, reactor_cell) =
            find_in_buffer(&buffer, "Reactor").expect("Reactor brand segment");
        assert_ne!(sun_cell.fg, reactor_cell.fg);

        // 2. Effects Off: compact brand renders immediately in its settled state.
        model.config.tui.effects = crate::config::MotionLevel::Off;
        model.motion.level = crate::config::MotionLevel::Off;
        let backend_off = TestBackend::new(85, 30);
        let mut terminal_off = Terminal::new(backend_off).unwrap();
        terminal_off.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_off = terminal_off.backend().buffer().clone();
        assert!(find_in_buffer(&buffer_off, "SunReactor").is_some());

        // 3. Compact mode: does not crash and renders clean title
        let backend_compact = TestBackend::new(65, 18);
        let mut terminal_compact = Terminal::new(backend_compact).unwrap();
        assert!(terminal_compact.draw(|f| ui::ui(f, &mut model)).is_ok());

        // 4. SemanticStyles styles verify two distinct colors for logo
        let palette = model.config.tui.theme.palette();
        let styles = palette.styles();
        assert_ne!(styles.chrome_title, styles.chrome_title_secondary);
    }

    #[test]
    fn test_phase9_3_copy_regression_and_forbidden_terms() {
        let mut model = Model::new();
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test across all tabs
        for tab in [
            Tab::Monitors,
            Tab::Limits,
            Tab::Location,
            Tab::Weather,
            Tab::Settings,
        ] {
            model.active_tab = tab;
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            // 1. Prohibited theatrical, redundant, or architectural jargon strings
            assert!(
                find_in_buffer(&buffer, "ATMOSPHERIC SUBSYSTEM").is_none(),
                "Found ATMOSPHERIC SUBSYSTEM on {tab:?}"
            );
            assert!(
                find_in_buffer(&buffer, "ATMOSPHERIC TELEMETRY").is_none(),
                "Found ATMOSPHERIC TELEMETRY on {tab:?}"
            );
            assert!(
                find_in_buffer(&buffer, "LOCATION IDENTITY").is_none(),
                "Found LOCATION IDENTITY on {tab:?}"
            );
            assert!(
                find_in_buffer(&buffer, "DRIVES AUTOMATION").is_none(),
                "Found DRIVES AUTOMATION on {tab:?}"
            );
            assert!(
                find_in_buffer(&buffer, "OPERATOR REFERENCE").is_none(),
                "Found OPERATOR REFERENCE on {tab:?}"
            );
            assert!(
                find_in_buffer(&buffer, "INSTRUMENT LEGEND").is_none(),
                "Found INSTRUMENT LEGEND on {tab:?}"
            );
            assert!(
                find_in_buffer(&buffer, "Dim automatically after idle").is_none(),
                "Found Dim automatically after idle on {tab:?}"
            );
            assert!(
                find_in_buffer(&buffer, "OpenWeather API key").is_none(),
                "Found OpenWeather API key on {tab:?}"
            );
            assert!(
                find_in_buffer(&buffer, "Solar automation is driving monitor brightness").is_none(),
                "Found paragraph slop on {tab:?}"
            );
        }
    }
}
