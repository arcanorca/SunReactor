use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::Model;

use super::{panel_border, truncate};
use crate::tui::theme::Palette;

pub(super) fn render_monitors(f: &mut Frame, app: &Model, area: Rect) {
    let status = if let Some(status) = &app.status {
        status
    } else {
        f.render_widget(
            Paragraph::new(" Connecting to daemon…")
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    };

    let palette = app.config.tui.theme.palette();
    if status.monitors.is_empty() {
        f.render_widget(
            Paragraph::new(" No monitors detected. Run `sunreactorctl discover` first.")
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(palette.border_inactive)),
                ),
            area,
        );
        return;
    }

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let items = status
        .monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let selected = index == app.selected_monitor;
            let applied = monitor
                .override_percent
                .unwrap_or(monitor.last_applied_percent.unwrap_or(0));
            let mode_tag = if monitor.override_percent.is_some() {
                "ovr"
            } else {
                "auto"
            };
            let (minimum, maximum, gamma) = monitor_limits(app, &monitor.logical_id);
            let summary_row = format!(
                " {} {:20} {:4} {:3}%",
                if selected { "▸" } else { " " },
                truncate(&monitor.logical_id, 20),
                mode_tag,
                applied,
            );
            let limit_row = format!("   limits {minimum}%-{maximum}%  |  gamma {gamma:.1}");

            let style = if selected {
                Style::default()
                    .bg(palette.accent)
                    .fg(palette.bg)
                    .add_modifier(Modifier::BOLD)
            } else if monitor.override_percent.is_some() {
                Style::default().fg(palette.warning)
            } else {
                Style::default().fg(palette.fg)
            };

            ListItem::new(vec![
                Line::from(summary_row),
                Line::from(limit_row),
                Line::from(""),
            ])
            .style(style)
        })
        .collect::<Vec<_>>();

    let header_line = format!("   {:20} {:4} {:>4}", "name", "mode", "appl");
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(panel_border(true, &palette))
            .title(ratatui::text::Span::styled(
                header_line,
                Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(list, columns[0]);

    if let Some(monitor) = status.monitors.get(app.selected_monitor) {
        render_monitor_detail(f, app, monitor, columns[1], &palette);
    }
}

fn render_monitor_detail(
    f: &mut Frame,
    app: &Model,
    monitor: &crate::ipc::MonitorStatus,
    area: Rect,
    palette: &Palette,
) {
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border(false, palette))
        .title(ratatui::text::Span::styled(
            format!(" {} ", monitor.logical_id),
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(detail_block.clone(), area);
    let inner = detail_block.inner(area);

    let detail_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(2),
            Constraint::Length(if app.monitor_advanced_open { 13 } else { 3 }),
            Constraint::Min(0),
        ])
        .split(inner);

    let applied = monitor
        .override_percent
        .unwrap_or(monitor.last_applied_percent.unwrap_or(0));

    let (mode_label, mode_style) = if let Some(override_percent) = monitor.override_percent {
        (
            format!("override ({override_percent}%)"),
            Style::default()
                .fg(palette.warning)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (String::from("auto"), Style::default().fg(palette.success))
    };

    let monitor_config = app
        .config
        .monitors
        .iter()
        .find(|m| m.logical_id == monitor.logical_id);
    let (minimum, maximum, _) = monitor_config.map_or((0, 100, 0.5), |m| {
        (m.min_pct, m.max_pct, m.transition_gamma)
    });
    let gamma = monitor_limits(app, &monitor.logical_id).2;

    let info_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{:10}", "Backend"),
                Style::default().fg(palette.text_muted),
            ),
            Span::styled(
                format!("{:?}", monitor.backend),
                Style::default().fg(palette.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:10}", "Mode"),
                Style::default().fg(palette.text_muted),
            ),
            Span::styled(mode_label, mode_style),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:10}", "Applied"),
                Style::default().fg(palette.text_muted),
            ),
            Span::styled(
                format!("{applied}%"),
                Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:10}", "Limits"),
                Style::default().fg(palette.text_muted),
            ),
            Span::styled(
                format!("{minimum}%-{maximum}%"),
                Style::default().fg(palette.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:10}", "Gamma"),
                Style::default().fg(palette.text_muted),
            ),
            Span::styled(
                format!("{gamma:.1} (+/- adjust)"),
                Style::default().fg(palette.fg),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(info_lines).wrap(ratatui::widgets::Wrap { trim: true }),
        detail_rows[0],
    );

    let gauge_percent = u16::from(applied);

    let gauge_color = if monitor.override_percent.is_some() {
        palette.warning
    } else {
        palette.accent
    };

    let gauge_title = format!(" brightness: {gauge_percent}% ");

    let gauge = ratatui::widgets::LineGauge::default()
        .block(
            Block::default()
                .title(gauge_title)
                .borders(Borders::TOP)
                .border_style(Style::default().fg(palette.border_inactive)),
        )
        .gauge_style(Style::default().fg(gauge_color))
        .line_set(ratatui::symbols::line::THICK)
        .label(Line::from(format!("{gauge_percent}%")))
        .ratio(f64::from(gauge_percent) / 100.0);
    f.render_widget(gauge, detail_rows[1]);

    render_monitor_advanced(f, app, &monitor.logical_id, detail_rows[2], palette);
}

fn monitor_limits(app: &Model, logical_id: &str) -> (u8, u8, f64) {
    app.config
        .monitors
        .iter()
        .find(|monitor| monitor.logical_id == logical_id)
        .map_or((0, 100, 0.5), |monitor| {
            (monitor.min_pct, monitor.max_pct, monitor.transition_gamma)
        })
}

fn render_monitor_advanced(
    f: &mut Frame,
    app: &Model,
    logical_id: &str,
    area: Rect,
    palette: &Palette,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border(false, palette))
        .title(ratatui::text::Span::styled(
            " Automation Milestones ",
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    if !app.monitor_advanced_open {
        f.render_widget(
            Paragraph::new(" Press [a] to inspect and fine-tune this monitor's daily milestones.")
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(palette.text_muted)),
            inner,
        );
        return;
    }

    if let Some(error) = &app.monitor_milestone_error {
        f.render_widget(
            Paragraph::new(format!(" Preview unavailable: {error}"))
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(palette.error)),
            inner,
        );
        return;
    }

    let Some(schedule) = app
        .monitor_milestones
        .iter()
        .find(|schedule| schedule.logical_id == logical_id)
    else {
        f.render_widget(
            Paragraph::new(" No milestone preview is available for this monitor.")
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(palette.text_muted)),
            inner,
        );
        return;
    };

    let selected_index = app
        .selected_monitor_milestone
        .min(schedule.milestones.len().saturating_sub(1));
    let current_index = current_milestone_index(schedule);
    let items = schedule
        .milestones
        .iter()
        .enumerate()
        .map(|(index, milestone)| {
            let selected = index == selected_index;
            let current = current_index == Some(index);
            let marker = if selected {
                "▸"
            } else if current {
                "•"
            } else {
                " "
            };
            let time_str = if app.config.tui.use_12h_time {
                milestone.adjusted_time_local.format("%I:%M %p").to_string()
            } else {
                milestone.adjusted_time_local.format("%H:%M").to_string()
            };
            let line = format!(
                " {} {:11} {:>8} -> {:3}% {:>5}",
                marker,
                milestone.milestone.label(),
                time_str,
                milestone.target_percent,
                offset_label(milestone.minutes_offset),
            );
            let style = if selected {
                Style::default()
                    .bg(palette.accent)
                    .fg(palette.bg)
                    .add_modifier(Modifier::BOLD)
            } else if current {
                Style::default()
                    .fg(palette.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.fg)
            };

            ListItem::new(Line::from(line)).style(style)
        })
        .collect::<Vec<_>>();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);
    let mut state = ListState::default();
    state.select(Some(selected_index));
    f.render_stateful_widget(List::new(items), rows[0], &mut state);

    let footer = schedule
        .milestones
        .get(selected_index)
        .map(|milestone| {
            if milestone.minutes_offset == 0 {
                String::from(" \u{2190}\u{2192} select  |  +/- 1m  |  r reset")
            } else {
                let base_time_str = if app.config.tui.use_12h_time {
                    milestone.base_time_local.format("%I:%M %p").to_string()
                } else {
                    milestone.base_time_local.format("%H:%M").to_string()
                };
                format!(" base {base_time_str}  |  \u{2190}\u{2192} select  |  +/- 1m  |  r reset")
            }
        })
        .unwrap_or_else(|| String::from(" \u{2190}\u{2192} select  |  +/- 1m  |  r reset"));
    f.render_widget(
        Paragraph::new(footer)
            .style(Style::default().fg(palette.text_muted))
            .wrap(ratatui::widgets::Wrap { trim: true }),
        rows[1],
    );
}

fn current_milestone_index(schedule: &crate::policy::MonitorMilestoneSchedule) -> Option<usize> {
    let now = chrono::Utc::now();
    schedule
        .milestones
        .iter()
        .enumerate()
        .filter(|(_, milestone)| milestone.adjusted_time_local.with_timezone(&chrono::Utc) <= now)
        .map(|(index, _)| index)
        .next_back()
}

fn offset_label(minutes_offset: i16) -> String {
    if minutes_offset == 0 {
        String::from("base")
    } else {
        format!("{minutes_offset:+}m")
    }
}
