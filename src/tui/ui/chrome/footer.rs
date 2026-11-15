use std::time::Instant;

use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::kit;
use super::super::truncate;
use super::super::weather_model;
use crate::tui::command::UiCommandContext;
use crate::tui::model::{ActionState, ErrorCategory};
use crate::tui::theme::SemanticStyles;
use crate::tui::{Model, MotionLevel};

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
    truncate(message, width.saturating_sub(4))
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
        weather_model::weather_action_label(app)
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
