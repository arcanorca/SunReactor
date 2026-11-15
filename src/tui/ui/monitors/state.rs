use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use super::super::kit;
use crate::tui::model::MonitorWorkspaceState;
use crate::tui::theme::SemanticStyles;
use crate::tui::{Model, MonitorPaneFocus, Tab};

/// Shared empty-state presentation for Monitors and Automation, derived from
/// independent daemon, configuration, and discovery evidence.
#[allow(clippy::too_many_lines)]
pub(crate) fn render_monitor_workspace_state(
    f: &mut Frame,
    app: &Model,
    area: Rect,
    styles: &SemanticStyles,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let setup_focused = app.workspace_focused()
        && (matches!(app.active_tab, Tab::Monitors)
            && matches!(app.monitor_pane_focus, MonitorPaneFocus::List)
            || matches!(app.active_tab, Tab::Limits));
    let action = |label: String| {
        Line::from(vec![
            kit::cursor_span(setup_focused, styles),
            Span::styled(
                label,
                if setup_focused {
                    styles.value_capsule_focused
                } else {
                    styles.data_value
                },
            ),
        ])
    };
    let mut lines = Vec::new();

    let title = match app.monitor_workspace_state() {
        MonitorWorkspaceState::DaemonUnavailable => {
            lines.push(Line::from(Span::styled(
                format!("{} Daemon unavailable", kit::HOLLOW),
                styles.status_error.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "SunReactor is not responding. Start the daemon, then return here.",
                styles.text_muted,
            )));
            "Monitors"
        }
        MonitorWorkspaceState::Connecting => {
            lines.push(Line::from(Span::styled(
                "Connecting to the SunReactor daemon…",
                styles.text_muted,
            )));
            "Monitors"
        }
        MonitorWorkspaceState::Discovering => {
            lines.push(Line::from(Span::styled(
                "Looking for compatible monitors…",
                styles.text_primary.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "Hardware discovery runs in the background.",
                styles.text_muted,
            )));
            "Monitors"
        }
        MonitorWorkspaceState::DiscoveredButUnconfigured {
            compatible_count,
            importable_count,
            incomplete_ddc_probe,
        } => {
            lines.push(Line::from(Span::styled(
                format!("{compatible_count} compatible monitor(s) found"),
                styles.text_primary.add_modifier(Modifier::BOLD),
            )));
            if let Some(report) = app.monitor_discovery_report() {
                for target in report.viable_targets().into_iter().take(4) {
                    lines.push(kit::detail_line(
                        vec![Span::styled(target.label.clone(), styles.text_primary)],
                        styles,
                    ));
                }
            }
            lines.push(Line::from(""));
            if importable_count > 0 && !incomplete_ddc_probe {
                lines.push(action(format!(" Add {importable_count} monitor(s) ")));
                lines.push(Line::from(Span::styled(
                    "Saves verified selectors and reloads the daemon.",
                    styles.text_muted,
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    if incomplete_ddc_probe {
                        "A DDC capability check was incomplete. Verify all displays before adding them."
                    } else {
                        "No candidate can be added automatically without weakening topology safety."
                    },
                    styles.status_warning,
                )));
                lines.push(action(String::from(" Scan again ")));
            }
            "Set up monitors"
        }
        MonitorWorkspaceState::NoCompatibleHardware => {
            lines.push(Line::from(Span::styled(
                "No compatible monitors were discovered",
                styles.text_primary.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "Connect a brightness-capable display, then scan again.",
                styles.text_muted,
            )));
            lines.push(Line::from(""));
            lines.push(action(String::from(" Scan again ")));
            "Set up monitors"
        }
        MonitorWorkspaceState::ConfiguredUnavailable => {
            lines.push(Line::from(Span::styled(
                format!(
                    "{} Configured monitors are temporarily unavailable",
                    kit::WARN
                ),
                styles.status_warning.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "Check display power and connections. Your configuration has not been changed.",
                styles.text_muted,
            )));
            "Monitors"
        }
        MonitorWorkspaceState::Ready => {
            lines.push(Line::from(Span::styled(
                "Monitor status is updating…",
                styles.text_muted,
            )));
            "Monitors"
        }
    };

    let block = kit::panel(title, setup_focused, styles);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .style(styles.base)
            .wrap(Wrap { trim: true })
            .block(block),
        area,
    );
}
