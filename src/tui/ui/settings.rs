use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use crate::tui::model::{settings_index as setting, DaemonLifecycle, InputMode};
use crate::tui::theme::SemanticStyles;
use crate::tui::Model;

use super::kit::{self, FieldKind, FieldState};

const LABEL_WIDTH: usize = 18;
const NARROW_LABEL_WIDTH: usize = 14;

fn label_width_for(width: u16) -> usize {
    if width >= 46 {
        LABEL_WIDTH
    } else {
        NARROW_LABEL_WIDTH
    }
}

/// One row of a settings group.
struct Row {
    index: Option<usize>,
    label: &'static str,
    value: String,
    kind: FieldKind,
}

impl Row {
    fn editable(index: usize, label: &'static str, value: String, kind: FieldKind) -> Self {
        Self {
            index: Some(index),
            label,
            value,
            kind,
        }
    }

    fn info(label: &'static str, value: String) -> Self {
        Self {
            index: None,
            label,
            value,
            kind: FieldKind::ReadOnly,
        }
    }
}

type Group = (&'static str, Vec<Row>);

/// Renders Settings (Tab 5): one vertical list in focus order, so ↑/↓ always
/// move to the row visually above or below, with a panel describing the
/// focused setting beside it when there is room.
pub(crate) fn render_settings(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let groups = setting_groups(app);
    let (list_area, about_area) = if area.width >= 96 {
        let list_width = (area.width * 11 / 20).clamp(56, 66);
        (
            Rect::new(area.x, area.y, list_width, area.height),
            Some(Rect::new(
                area.x + list_width + 1,
                area.y,
                area.width.saturating_sub(list_width + 1),
                area.height.min(12),
            )),
        )
    } else {
        (area, None)
    };

    render_list(f, app, &groups, list_area, styles);
    if let Some(about_area) = about_area {
        render_about(f, app, &groups, about_area, styles);
    }
}

#[allow(clippy::too_many_lines)]
fn setting_groups(app: &Model) -> Vec<Group> {
    let idle = if !app.config.daemon.desktop_idle_sync
        || app.config.daemon.desktop_idle_timeout_minutes == 0
    {
        String::from("Off")
    } else {
        format!("{} min", app.config.daemon.desktop_idle_timeout_minutes)
    };
    let suspend = if app.form.suspend_minutes_input.value().trim().is_empty() {
        String::from("Until resumed")
    } else {
        format!("{} min", app.form.suspend_minutes_input.value().trim())
    };
    let api_key = if app.form.api_key_input.value().trim().is_empty() {
        String::from("Not set")
    } else {
        String::from("●●●●●●●●")
    };

    let interface = vec![
        Row::editable(
            setting::THEME,
            "Theme",
            app.config.tui.theme.name().to_owned(),
            FieldKind::Text,
        ),
        Row::editable(
            setting::EFFECTS,
            "Effects",
            app.config.tui.effects.label().to_owned(),
            FieldKind::Adjustable,
        ),
        Row::editable(
            setting::SHOW_LOGO,
            "Large logo",
            on_off(app.config.tui.show_logo),
            FieldKind::Adjustable,
        ),
        Row::editable(
            setting::REFRESH_RATE,
            "Animation rate",
            format!("{} fps", app.config.tui.fps),
            FieldKind::Text,
        ),
        Row::editable(
            setting::TIME_FORMAT,
            "Time format",
            String::from(if app.config.tui.use_12h_time {
                "12-hour"
            } else {
                "24-hour"
            }),
            FieldKind::Adjustable,
        ),
        Row::editable(
            setting::TEMPERATURE_UNIT,
            "Temperature",
            String::from(match app.config.tui.temperature_unit {
                crate::config::TemperatureUnit::Celsius => "°C",
                crate::config::TemperatureUnit::Fahrenheit => "°F",
            }),
            FieldKind::Adjustable,
        ),
    ];
    let power = vec![
        Row::editable(setting::IDLE_DIM, "Dim when idle", idle, FieldKind::Text),
        Row::editable(
            setting::SUSPEND_DURATION,
            "Suspend for",
            suspend,
            FieldKind::Text,
        ),
    ];
    let weather = vec![
        Row::editable(
            setting::WEATHER_ENABLED,
            "Use weather",
            on_off(app.config.weather.enabled),
            FieldKind::Adjustable,
        ),
        Row::editable(
            setting::WEATHER_API_KEY,
            "API key",
            api_key,
            FieldKind::Text,
        ),
        Row::info("Provider", String::from("OpenWeather forecast")),
        Row::info(
            "Refresh",
            format!("Every {} min", app.config.weather.refresh_minutes),
        ),
    ];

    let suspended = app.status.as_ref().is_some_and(|status| status.suspended);
    let daemon = match app.daemon_lifecycle() {
        DaemonLifecycle::Active => String::from("Running"),
        DaemonLifecycle::IdleDimmed => String::from("Running · idle dimmed"),
        DaemonLifecycle::Suspended => String::from("Running · writes paused"),
        DaemonLifecycle::Unreachable => String::from("Not responding"),
    };
    let writes = if suspended {
        let until = app
            .status
            .as_ref()
            .and_then(|status| status.suspend_until_epoch_s);
        format!("Paused {}", format_suspend_until(until, true, app))
    } else {
        String::from("Enabled")
    };
    let service = vec![
        Row::info("Daemon", daemon),
        Row::info("Display writes", writes),
    ];

    vec![
        ("Interface", interface),
        ("Power", power),
        ("Weather", weather),
        ("Service", service),
    ]
}

fn on_off(value: bool) -> String {
    String::from(if value { "On" } else { "Off" })
}

fn edit_value(app: &Model, index: usize) -> Option<String> {
    (matches!(app.input_mode, InputMode::Editing) && app.active_setting == index)
        .then(|| app.active_input_ref().map(|input| input.value().to_owned()))
        .flatten()
}

fn row_line(app: &Model, row: &Row, width: usize, styles: &SemanticStyles) -> Line<'static> {
    let editing = matches!(app.input_mode, InputMode::Editing);
    let focused = row.index == Some(app.active_setting) && app.workspace_focused();
    let label_width = label_width_for(width as u16);
    let value_width = width.saturating_sub(2 + label_width + 1 + 4);
    let edit = row
        .index
        .and_then(|index| edit_value(app, index))
        .map(|value| {
            if row.index == Some(setting::WEATHER_API_KEY) {
                // Secrets stay masked while typing; only the length is visible.
                "●".repeat(value.chars().count())
            } else {
                value
            }
        });
    let mut line = kit::field_line(
        row.label,
        label_width,
        &super::truncate(&row.value, value_width),
        row.kind,
        FieldState::new(focused, focused && editing),
        edit.as_deref(),
        styles,
    );
    if row.label == "Daemon" || row.label == "Display writes" {
        // Service state keeps a status glyph so health reads at a glance.
        let healthy = !row.value.starts_with("Not") && !row.value.starts_with("Paused");
        let glyph = if healthy {
            Span::styled(format!("{} ", kit::DOT), styles.status_success)
        } else {
            Span::styled(format!("{} ", kit::WARN), styles.status_warning)
        };
        let value = line.spans.pop();
        line.spans.push(glyph);
        if let Some(value) = value {
            line.spans.push(value.style(styles.text_primary));
        }
    }
    line
}

/// All groups in one panel. The list scrolls to keep the focused row visible.
fn render_list(f: &mut Frame, app: &Model, groups: &[Group], area: Rect, styles: &SemanticStyles) {
    let block = kit::panel("Settings", app.workspace_focused(), styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut lines: Vec<(Line, bool)> = Vec::new();
    for (position, (title, rows)) in groups.iter().enumerate() {
        if position > 0 {
            lines.push((Line::from(""), false));
        }
        lines.push((kit::heading(title, inner.width, styles), false));
        for row in rows {
            lines.push((
                row_line(app, row, usize::from(inner.width), styles),
                row.index == Some(app.active_setting),
            ));
        }
    }
    let focus_row = lines.iter().position(|(_, focused)| *focused).unwrap_or(0);
    let visible = usize::from(inner.height);
    let start = focus_row
        .saturating_sub(visible.saturating_sub(3))
        .min(lines.len().saturating_sub(visible));
    let shown: Vec<Line> = lines
        .into_iter()
        .skip(start)
        .take(visible)
        .map(|(line, _)| line)
        .collect();
    f.render_widget(Paragraph::new(shown), inner);

    if matches!(app.input_mode, InputMode::Editing) && focus_row >= start {
        if let Some(input) = app.active_input_ref() {
            let row = Rect::new(
                inner.x,
                inner.y + (focus_row - start) as u16,
                inner.width,
                1,
            );
            if row.y < inner.y + inner.height {
                let slice = compute_horizontal_viewport(input.value(), input.cursor(), 32);
                kit::place_field_cursor(f, row, label_width_for(inner.width), slice.cursor_col);
            }
        }
    }

    if let Some(error) = &app.config_error {
        if area.y + area.height < f.size().height {
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!(" ✕ {error}"),
                    styles.status_error.add_modifier(Modifier::BOLD),
                )),
                Rect::new(area.x, area.y + area.height, area.width, 1),
            );
        }
    }
}

/// What the focused setting does, and its choices when there are few.
fn render_about(f: &mut Frame, app: &Model, groups: &[Group], area: Rect, styles: &SemanticStyles) {
    let Some(row) = groups
        .iter()
        .flat_map(|(_, rows)| rows)
        .find(|row| row.index == Some(app.active_setting))
    else {
        return;
    };
    let block = kit::panel(row.label, false, styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);

    let max_dim = ((1.0 - app.config.weather.min_multiplier) * 100.0).round();
    let description = match app.active_setting {
        setting::THEME => String::from("Colour palette for every screen. Enter opens the list."),
        setting::EFFECTS => String::from(
            "Full adds transitions and live feedback. Reduced keeps small confirmations. Off removes motion.",
        ),
        setting::SHOW_LOGO => {
            String::from("Shows the SunReactor wordmark when the terminal is at least 76×32.")
        }
        setting::REFRESH_RATE => String::from(
            "Frame rate while an animation runs, used between 24 and 60. Idle screens redraw once per second.",
        ),
        setting::TIME_FORMAT => String::from("Clock and schedule times."),
        setting::TEMPERATURE_UNIT => String::from("Weather temperatures."),
        setting::IDLE_DIM => String::from(
            "Dims displays after this many minutes without desktop input. 0 turns it off.",
        ),
        setting::SUSPEND_DURATION => String::from(
            "How long s pauses brightness writes. Leave it empty to pause until you resume with r.",
        ),
        setting::WEATHER_ENABLED => {
            format!("Cloud cover can lower daylight brightness by up to {max_dim:.0}%.")
        }
        setting::WEATHER_API_KEY => String::from(
            "Your OpenWeather key. Saved in config.toml unless it comes from an environment variable. Never shown.",
        ),
        _ => String::new(),
    };
    let choices: &[&str] = match app.active_setting {
        setting::EFFECTS => &["Full", "Reduced", "Off"],
        setting::SHOW_LOGO | setting::WEATHER_ENABLED => &["On", "Off"],
        setting::TIME_FORMAT => &["24-hour", "12-hour"],
        setting::TEMPERATURE_UNIT => &["°C", "°F"],
        _ => &[],
    };

    let mut lines = vec![Line::from(Span::styled(description, styles.text_primary))];
    if !choices.is_empty() {
        lines.push(Line::from(""));
        let mut spans = Vec::new();
        for (position, choice) in choices.iter().enumerate() {
            if position > 0 {
                spans.push(Span::styled(kit::SEPARATOR, styles.border_normal));
            }
            spans.push(if *choice == row.value {
                Span::styled(
                    format!("{} {choice}", kit::DOT),
                    styles.focus_marker.add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(format!("{} {choice}", kit::HOLLOW), styles.text_muted)
            });
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn format_suspend_until(until_epoch_s: Option<u64>, suspended: bool, app: &Model) -> String {
    if !suspended {
        return String::from("not suspended");
    }

    let Some(epoch_s) = until_epoch_s else {
        return String::from("until resume");
    };

    let Some(dt) = app.local_time_at_epoch(epoch_s) else {
        return format!("epoch {epoch_s}");
    };
    let format_str = if app.config.tui.use_12h_time {
        "%I:%M %p"
    } else {
        "%H:%M"
    };
    let absolute = dt.format(format_str).to_string();

    let now_epoch_s = chrono::Utc::now().timestamp().max(0) as u64;
    let remaining_minutes = epoch_s.saturating_sub(now_epoch_s).div_ceil(60);
    format!("until {absolute} ({remaining_minutes} min left)")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ViewportSlice {
    pub display_text: String,
    pub cursor_col: usize,
}

pub(crate) fn compute_horizontal_viewport(
    value: &str,
    cursor_char_idx: usize,
    viewport_width: usize,
) -> ViewportSlice {
    let char_count = value.chars().count();
    let cursor = cursor_char_idx.min(char_count);

    if viewport_width == 0 {
        return ViewportSlice {
            display_text: String::new(),
            cursor_col: 0,
        };
    }

    if char_count <= viewport_width {
        return ViewportSlice {
            display_text: value.to_string(),
            cursor_col: cursor,
        };
    }

    let start_char = if cursor < viewport_width {
        0
    } else {
        let ideal_start = cursor.saturating_sub(viewport_width.saturating_sub(1));
        ideal_start.min(char_count.saturating_sub(viewport_width))
    };

    let slice: String = value
        .chars()
        .skip(start_char)
        .take(viewport_width)
        .collect();
    let cursor_col = cursor.saturating_sub(start_char);

    ViewportSlice {
        display_text: slice,
        cursor_col,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};

    use crate::tui::model::{settings_index, DaemonConnection, Tab};
    use crate::tui::test_support::{
        buffer_row, dummy_status, dummy_weather_status, find_in_buffer, press, two_monitor_model,
    };
    use crate::tui::ui;
    use crate::tui::Model;
    #[test]
    fn test_power_management_in_settings() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.active_tab = Tab::Settings;
        model.active_setting = settings_index::IDLE_DIM;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Dim when idle").is_some());
        assert!(find_in_buffer(&buffer, "Power").is_some());
    }

    #[test]
    fn test_phase7_settings_workspace_comfortable_layout() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.effects = crate::config::MotionLevel::Instrument;
        model.motion.level = crate::config::MotionLevel::Instrument;
        model.active_tab = Tab::Settings;
        model.active_setting = 0; // Theme
        model.daemon_connection = DaemonConnection::Connected;
        model.status = Some(dummy_status(2));

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        for group in ["Interface ─", "Power ─", "Weather ─", "Service ─"] {
            assert!(find_in_buffer(&buffer, group).is_some(), "{group} missing");
        }
        for label in [
            "Theme",
            "Effects",
            "Animation rate",
            "Time format",
            "Temperature",
            "Dim when idle",
            "Suspend for",
            "Daemon",
            "Display writes",
        ] {
            assert!(find_in_buffer(&buffer, label).is_some(), "{label} missing");
        }
        assert!(find_in_buffer(&buffer, "Running").is_some());
        assert!(find_in_buffer(&buffer, "Enabled").is_some());
        assert!(find_in_buffer(&buffer, model.config.tui.theme.name()).is_some());
        assert!(find_in_buffer(&buffer, "Full").is_some());
        assert!(find_in_buffer(&buffer, "24-hour").is_some());
        assert!(find_in_buffer(&buffer, "°C").is_some());

        // The panel beside the list explains the focused setting.
        assert!(find_in_buffer(&buffer, "Colour palette for every screen").is_some());

        // The about panel's title also says "Theme", so locate the list rows by
        // their cursor gutter.
        assert!(find_in_buffer(&buffer, "❯ Theme").is_some());
        assert!(find_in_buffer(&buffer, "❯ Effects").is_none());
        assert!(find_in_buffer(&buffer, "  Effects").is_some());
    }

    #[test]
    fn test_phase7_settings_power_management_semantic_display() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.effects = crate::config::MotionLevel::Instrument;
        model.motion.level = crate::config::MotionLevel::Instrument;
        model.active_tab = Tab::Settings;
        model.config.daemon.desktop_idle_sync = false;
        model.config.daemon.desktop_idle_timeout_minutes = 0;
        model.form.desktop_idle_timeout_minutes_input =
            tui_input::Input::default().with_value(String::from("0"));
        model.form.suspend_minutes_input = tui_input::Input::default();

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Zero idle minutes displays as Off without min unit
        assert!(find_in_buffer(&buffer, "Off").is_some());
        assert!(find_in_buffer(&buffer, "Until resume").is_some());

        // Configure 15 minutes idle and 45 minutes suspend
        model.config.daemon.desktop_idle_sync = true;
        model.config.daemon.desktop_idle_timeout_minutes = 15;
        model.form.desktop_idle_timeout_minutes_input =
            tui_input::Input::default().with_value(String::from("15"));
        model.form.suspend_minutes_input =
            tui_input::Input::default().with_value(String::from("45"));

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer2 = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer2, "15").is_some());
        assert!(find_in_buffer(&buffer2, "45").is_some());
        assert!(find_in_buffer(&buffer2, "min").is_some());
    }

    #[test]
    fn test_phase7_settings_responsive_compact_and_minimal() {
        // 1. Compact (65x20)
        {
            let backend = TestBackend::new(65, 20);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.effects = crate::config::MotionLevel::Instrument;
            model.motion.level = crate::config::MotionLevel::Instrument;
            model.active_tab = Tab::Settings;
            model.active_setting = settings_index::REFRESH_RATE;
            model.daemon_connection = DaemonConnection::Connected;
            model.status = Some(dummy_status(1));

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Interface").is_some());
            assert!(find_in_buffer(&buffer, "Power").is_some());
            assert!(find_in_buffer(&buffer, "Effects").is_some());
            let (rx, ry, _) =
                find_in_buffer(&buffer, "Animation rate").expect("focused row visible");
            assert_eq!(buffer.get(rx - 2, ry).symbol(), "❯");
        }

        // 2. Minimal (50x13)
        {
            let backend = TestBackend::new(50, 13);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.effects = crate::config::MotionLevel::Instrument;
            model.motion.level = crate::config::MotionLevel::Instrument;
            model.active_tab = Tab::Settings;
            model.active_setting = 1; // Effects
            model.daemon_connection = DaemonConnection::Connected;
            model.status = Some(dummy_status(1));

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            // Focused setting is visible in sliding window
            assert!(find_in_buffer(&buffer, "Effects").is_some());
            let (ex, ey, _) = find_in_buffer(&buffer, "Effects").unwrap();
            assert_eq!(buffer.get(ex - 2, ey).symbol(), "❯");
        }
    }

    #[test]
    fn test_phase7_settings_themes_visual_hierarchy() {
        for theme in [
            crate::config::Theme::Amber,
            crate::config::Theme::Nord,
            crate::config::Theme::HackerGreen,
            crate::config::Theme::Grayscale,
            crate::config::Theme::Commodore64,
        ] {
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.theme = theme;
            model.config.tui.effects = crate::config::MotionLevel::Instrument;
            model.motion.level = crate::config::MotionLevel::Instrument;
            model.active_tab = Tab::Settings;
            model.active_setting = 1; // Effects focused
            model.daemon_connection = DaemonConnection::Connected;
            model.status = Some(dummy_status(1));

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Interface").is_some());
            assert!(find_in_buffer(&buffer, "Effects").is_some());
            assert!(find_in_buffer(&buffer, "Full").is_some());
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Phase 8: Weather v2 — Atmospheric Control Instrument Tests
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_phase9_3_settings_layout_spacing_and_balance() {
        let mut model = Model::new();
        model.active_tab = Tab::Settings;
        model.daemon_connection = DaemonConnection::Connected;
        model.status = Some(dummy_status(1));

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        let heading = |text: &str| find_in_buffer(&buffer, text).unwrap();
        let (interface_x, interface_y, _) = heading("Interface ─");
        let (power_x, power_y, _) = heading("Power ─");
        let (weather_x, weather_y, _) = heading("Weather ─");
        let (service_x, service_y, _) = heading("Service ─");

        // One column in focus order: ↑/↓ never jump sideways.
        assert_eq!(interface_x, power_x);
        assert_eq!(power_x, weather_x);
        assert_eq!(weather_x, service_x);
        assert!(interface_y < power_y && power_y < weather_y && weather_y < service_y);

        // A blank row separates each group from the last row of the previous one.
        let (_, temperature_y, _) = find_in_buffer(&buffer, "Temperature").unwrap();
        assert_eq!(power_y, temperature_y + 2);
        assert!(
            buffer_row(&buffer, temperature_y + 1)[..usize::from(interface_x) + 40]
                .trim_matches(|c| c == ' ' || c == '│')
                .is_empty()
        );
    }

    #[test]
    fn test_settings_focus_moves_straight_down_one_column() {
        let mut model = two_monitor_model();
        model.active_tab = Tab::Settings;
        model.active_setting = 0;
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();

        let mut previous: Option<(u16, u16)> = None;
        for step in 0..settings_index::COUNT {
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();
            let (x, y, _) = find_in_buffer(&buffer, "❯ ").expect("focused row");
            if let Some((px, py)) = previous {
                assert_eq!(x, px, "step {step}: the cursor moved sideways");
                assert!(y > py, "step {step}: ↓ must move down");
            }
            previous = Some((x, y));
            press(&mut model, KeyCode::Down, KeyModifiers::NONE);
        }
    }
    #[test]
    fn test_editor_horizontal_viewport_clamping_and_scrolling() {
        use super::compute_horizontal_viewport;

        // Text fits completely in viewport
        let res = compute_horizontal_viewport("hello", 2, 10);
        assert_eq!(res.display_text, "hello");
        assert_eq!(res.cursor_col, 2);

        // Long text with cursor at start (Home)
        let res = compute_horizontal_viewport("0123456789ABCDEF", 0, 8);
        assert_eq!(res.display_text, "01234567");
        assert_eq!(res.cursor_col, 0);

        // Long text with cursor at end (End)
        let res = compute_horizontal_viewport("0123456789ABCDEF", 16, 8);
        assert_eq!(res.display_text, "89ABCDEF");
        assert_eq!(res.cursor_col, 8);

        // Long text with cursor in middle
        let res = compute_horizontal_viewport("0123456789ABCDEF", 10, 8);
        assert!(res.cursor_col <= 8);
        let expected_char = "0123456789ABCDEF".chars().nth(10).unwrap();
        // Cursor points to 'A' which must be in display_text
        assert!(res.display_text.contains(expected_char));

        // Unicode test
        let res = compute_horizontal_viewport("İstanbul/Türkiye", 10, 8);
        assert!(res.cursor_col <= 8);
        assert_eq!(res.display_text.chars().count(), 8);
    }

    #[test]
    fn test_phase7_settings_navigation_and_effects_toggle_live() {
        let mut model = Model::new();
        model.config.tui.effects = crate::config::MotionLevel::Instrument;
        model.motion.level = crate::config::MotionLevel::Instrument;
        model.active_tab = Tab::Settings;
        model.active_setting = 0;
        assert_eq!(model.tab_field_count(Tab::Settings), 10);

        // 1. Navigate down to Effects (index 1)
        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Down,
            )),
        );
        assert_eq!(model.active_setting, 1);
        assert_eq!(
            model.config.tui.effects,
            crate::config::MotionLevel::Instrument
        );
        assert_eq!(model.motion.level, crate::config::MotionLevel::Instrument);

        // 2. Press Enter to cycle to Reduced
        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Enter,
            )),
        );
        assert_eq!(
            model.config.tui.effects,
            crate::config::MotionLevel::Reduced
        );
        assert_eq!(model.motion.level, crate::config::MotionLevel::Reduced);

        // 3. Press Space to cycle to Off
        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(' '),
            )),
        );
        assert_eq!(model.config.tui.effects, crate::config::MotionLevel::Off);
        assert_eq!(model.motion.level, crate::config::MotionLevel::Off);
        assert!(model.motion.active_transient.is_none());

        // 4. Press Right to cycle back to Instrument
        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Right,
            )),
        );
        assert_eq!(
            model.config.tui.effects,
            crate::config::MotionLevel::Instrument
        );
        assert_eq!(model.motion.level, crate::config::MotionLevel::Instrument);

        // 5. Press Left to cycle backwards to Off
        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Left,
            )),
        );
        assert_eq!(model.config.tui.effects, crate::config::MotionLevel::Off);
        assert_eq!(model.motion.level, crate::config::MotionLevel::Off);
    }

    #[test]
    fn test_phase7_settings_operational_state_and_actions() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.effects = crate::config::MotionLevel::Instrument;
        model.motion.level = crate::config::MotionLevel::Instrument;
        model.active_tab = Tab::Settings;

        // 1. Disconnected/Offline state
        model.daemon_connection = crate::tui::model::DaemonConnection::Disconnected;
        model.status = None;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_offline = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer_offline, "Offline").is_some());
        assert!(find_in_buffer(&buffer_offline, "daemon service unreachable").is_none());

        // 2. Connected and Suspended state
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.suspended = true;
        status.suspend_until_epoch_s = Some(1_800_000_000);
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_suspended = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer_suspended, "Suspended").is_some());

        // 3. Suspend & Resume key triggers on Tab::Settings
        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('s'),
            )),
        );
        assert!(matches!(
            model.action_state,
            crate::tui::model::ActionState::Pending {
                action: crate::tui::model::ActionKind::Suspend,
                ..
            }
        ));

        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('r'),
            )),
        );
        assert!(matches!(
            model.action_state,
            crate::tui::model::ActionState::Pending {
                action: crate::tui::model::ActionKind::Resume,
                ..
            }
        ));
    }

    #[test]
    fn test_phase8_2_settings_atmosphere_consolidation_and_editing() {
        let mut model = Model::new();
        model.active_tab = Tab::Settings;
        model.active_setting = crate::tui::model::settings_index::WEATHER_ENABLED;
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.status = Some(dummy_status(1));

        // 1. Settings shows ATMOSPHERE section with all 4 rows
        {
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "Weather ─").is_some());
            assert!(find_in_buffer(&buffer, "Use weather").is_some());
            assert!(find_in_buffer(&buffer, "Provider").is_some());
            assert!(find_in_buffer(&buffer, "OpenWeather").is_some());
            assert!(find_in_buffer(&buffer, "Refresh").is_some());
            assert!(find_in_buffer(&buffer, "30 min").is_some());
            assert!(find_in_buffer(&buffer, "API key").is_some());
        }

        // 2. Weather Tab is purely observational: field count is 0, no API key row
        {
            assert_eq!(model.tab_field_count(Tab::Weather), 0);
            model.active_tab = Tab::Weather;
            model.config.weather.enabled = true;
            let mut status = dummy_status(1);
            status.weather = Some(dummy_weather_status(
                true,
                true,
                false,
                Some(50),
                Some(0.8),
                8,
            ));
            model.status = Some(status);

            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "OpenWeather API key").is_none());
            assert!(find_in_buffer(&buffer, "OpenWeather API Key").is_none());
            assert!(find_in_buffer(&buffer, "Up to date").is_some());
            assert!(find_in_buffer(&buffer, "Current weather").is_none());
        }

        // 3. Toggle Weather enabled in Settings
        {
            model.active_tab = Tab::Settings;
            model.active_setting = crate::tui::model::settings_index::WEATHER_ENABLED;
            let initial_enabled = model.config.weather.enabled;

            crate::tui::update::update(
                &mut model,
                crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Enter,
                )),
            );
            assert_eq!(model.config.weather.enabled, !initial_enabled);
        }

        // 4. API Key editing and cancel/save in Settings
        {
            model.active_setting = crate::tui::model::settings_index::WEATHER_API_KEY;
            crate::tui::update::update(
                &mut model,
                crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Enter,
                )),
            );
            assert!(matches!(
                model.input_mode,
                crate::tui::model::InputMode::Editing
            ));

            // Type new secret
            crate::tui::update::update(
                &mut model,
                crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Char('k'),
                )),
            );

            // Cancel with Esc
            crate::tui::update::update(
                &mut model,
                crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::Esc,
                )),
            );
            assert!(matches!(
                model.input_mode,
                crate::tui::model::InputMode::Normal
            ));
        }
    }
}
