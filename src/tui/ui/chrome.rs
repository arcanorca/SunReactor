use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame,
};

use crate::tui::{InputMode, Model, Tab};

use crate::tui::theme::{focused_style, Palette};

pub(super) fn render_header(f: &mut Frame, app: &Model, area: Rect) {
    let palette = app.config.tui.theme.palette();
    // Build status strings regardless of display mode.
    let (daemon, weather) = if let Some(status) = &app.status {
        let daemon = if status.suspended {
            "suspended"
        } else if status.desktop_idle_dimmed {
            "idle dimmed"
        } else if status.daemon_alive {
            "active"
        } else {
            "unreachable"
        };
        let weather = match &status.weather {
            Some(weather) if weather.active => {
                format!("{}% ☁", weather.cloud_cover_percent.unwrap_or(0))
            }
            Some(weather) if weather.stale => String::from("stale"),
            Some(weather) if weather.last_error.is_some() => String::from("error"),
            Some(_) => String::from("idle"),
            None => String::from("off"),
        };
        (daemon.to_string(), weather)
    } else {
        (String::from("unreachable"), String::from("…"))
    };

    let timezone_str = &app.config.location.timezone;
    let now_utc = chrono::Utc::now();
    let path = std::path::Path::new("/usr/share/zoneinfo").join(timezone_str);
    let offset = std::fs::read(&path)
        .ok()
        .and_then(|data| tz::TimeZone::from_tz_data(&data).ok())
        .or_else(|| tz::TimeZone::from_posix_tz(timezone_str).ok())
        .and_then(|tz| {
            tz.find_local_time_type(now_utc.timestamp())
                .map(|lt| lt.ut_offset())
                .ok()
        })
        .map(|offset| chrono::FixedOffset::east_opt(offset).unwrap());

    let time_str = match offset {
        Some(off) => {
            let now = now_utc.with_timezone(&off);
            if app.config.tui.use_12h_time {
                now.format("%I:%M:%S %p").to_string()
            } else {
                now.format("%H:%M:%S").to_string()
            }
        }
        None => {
            if app.config.tui.use_12h_time {
                chrono::Local::now().format("%I:%M:%S %p").to_string()
            } else {
                chrono::Local::now().format("%H:%M:%S").to_string()
            }
        }
    };

    let (badge_text, badge_bg) = if let Some(status) = app.status.as_ref() {
        if status.weather.as_ref().is_some_and(|w| w.active) {
            (" LIVE ", palette.success)
        } else {
            (" OFFLINE ", palette.error)
        }
    } else {
        (" OFFLINE ", palette.error)
    };

    let api_badge = Span::styled(
        badge_text,
        Style::default()
            .fg(palette.bg)
            .bg(badge_bg)
            .add_modifier(Modifier::BOLD),
    );

    let status_line = Line::from(vec![
        Span::styled(
            format!(" daemon:{daemon} "),
            style_daemon_status(&daemon, &palette),
        ),
        Span::raw(format!(" {time_str}  wthr:{weather}  ")),
        api_badge,
    ]);

    // ── Large logo (tall terminal ≥ 30 rows) ────────────────────────────────
    if area.height >= 7 {
        let logo_lines = [
            (
                r"  ____              ____                 _            ",
                palette.accent,
            ),
            (
                r" / ___| _   _ _ __ |  _ \ ___  __ _  ___| |_ ___  _ __",
                palette.accent,
            ),
            (
                r" \___ \| | | | '_ \| |_) / _ \/ _` |/ __| __/ _ \| '__|",
                palette.secondary_accent,
            ),
            (
                r"  ___) | |_| | | | |  _ <  __/ (_| | (__| || (_) | |   ",
                palette.accent,
            ),
            (
                r" |____/ \__,_|_| |_|_| \_\___|\__,_|\___|\__\___/|_|   ",
                palette.accent,
            ),
        ];

        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(palette.border_inactive));
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(block.inner(area));

        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(
                logo_lines
                    .iter()
                    .map(|(text, color)| {
                        Line::from(Span::styled(
                            *text,
                            Style::default().fg(*color).add_modifier(Modifier::BOLD),
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
            .alignment(Alignment::Center),
            layout[0],
        );
        f.render_widget(
            Paragraph::new(status_line)
                .alignment(Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true }),
            layout[1],
        );
    } else {
        // ── Compact single-line title bar (small terminal) ───────────────────
        // Left: compact app name; Right: status line
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(palette.border_inactive));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(inner);

        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                " ☀ SunReactor",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )]))
            .alignment(Alignment::Left),
            columns[0],
        );
        f.render_widget(
            Paragraph::new(status_line).alignment(Alignment::Right),
            columns[1],
        );
    }
}

pub(super) fn render_tabs(f: &mut Frame, app: &Model, area: Rect) {
    let palette = app.config.tui.theme.palette();
    let tabs = Tabs::new(
        Tab::ALL
            .iter()
            .map(|tab| Line::from(tab.title()))
            .collect::<Vec<_>>(),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.border_active)),
    )
    .select(app.active_tab.index())
    .highlight_style(focused_style(&palette, false));
    f.render_widget(tabs, area);
}

pub(super) fn render_footer(f: &mut Frame, app: &Model, area: Rect) {
    let palette = app.config.tui.theme.palette();
    let text = match app.active_tab {
        Tab::Monitors => {
            if app.monitor_advanced_open {
                " ↑↓ milestone  |  ←→ ±1m  |  a close  |  Tab switch  |  ? help  |  q quit"
                    .to_string()
            } else {
                " ↑↓ select  |  s suspend  |  r resume  |  Tab switch  |  ? help  |  q quit"
                    .to_string()
            }
        }
        Tab::Limits | Tab::Location | Tab::Weather => match app.input_mode {
            InputMode::Normal => {
                " ↑↓ select  |  Enter edit  |  Tab switch  |  ? help  |  q quit".to_string()
            }
            InputMode::Editing => " type to edit  |  Enter done  |  Esc cancel".to_string(),
        },
        Tab::Settings => match app.input_mode {
            InputMode::Normal => {
                " ↑↓ select  |  Enter edit  |  s suspend  |  r resume  |  Tab switch  |  ? help  |  q quit".to_string()
            }
            InputMode::Editing => " digits only  |  Enter done  |  Esc cancel".to_string(),
        },
    };

    f.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .style(Style::default().fg(palette.text_muted).bg(palette.bg))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(palette.border_inactive)),
            ),
        area,
    );
}

pub(super) fn render_help(f: &mut Frame, palette: &Palette) {
    let area = centered_rect(60, 50, f.size());
    let lines = vec![
        Line::from(Span::styled(
            " GLOBAL ",
            Style::default().add_modifier(Modifier::REVERSED),
        )),
        Line::from("  Tab / ← →    switch tabs"),
        Line::from("               (Monitors advanced uses ← → locally)"),
        Line::from("  s            suspend writes (global)"),
        Line::from("  r            resume writes (global)"),
        Line::from("  q            quit"),
        Line::from("  ?            toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            " MONITORS ",
            Style::default().add_modifier(Modifier::REVERSED),
        )),
        Line::from("  ↑ ↓          select monitor (or milestone in advanced)"),
        Line::from("  a            toggle automation advanced"),
        Line::from("  s / r        suspend / resume daemon"),
        Line::from("  ← / →        adjust milestone ±1m"),
        Line::from(""),
        Line::from(Span::styled(
            " Editing (Limits, Location, Weather, Settings) ",
            Style::default().add_modifier(Modifier::REVERSED),
        )),
        Line::from("  ↑ ↓          select field"),
        Line::from("  Enter        start editing"),
        Line::from("  Esc          stop editing"),
    ];

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .block(
                Block::default()
                    .title(" Keybindings ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.border_active)),
            ),
        area,
    );
}

fn style_daemon_status(status: &str, palette: &Palette) -> Style {
    match status {
        "active" => Style::default().fg(palette.success),
        "idle dimmed" => Style::default()
            .fg(palette.warning)
            .add_modifier(Modifier::BOLD),
        "suspended" => Style::default()
            .fg(palette.error)
            .add_modifier(Modifier::BOLD),
        _ => Style::default().fg(palette.error),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, rect: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(rect);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
