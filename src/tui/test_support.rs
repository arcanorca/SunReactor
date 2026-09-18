//! Shared fixtures for TUI tests that exercise more than one owner.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::{Buffer, Cell};

use crate::{
    backends::BackendKind,
    config::{Config, MonitorConfig, MonitorSelector},
    ipc::{MonitorStatus, StatusResponse},
    tui::{
        form::FormState,
        model::DaemonConnection,
        update::{self, Message},
        Model,
    },
};

pub(crate) fn dummy_status(monitor_count: usize) -> StatusResponse {
    let monitors = (0..monitor_count)
        .map(|index| MonitorStatus {
            logical_id: format!("mon-{index}"),
            backend: BackendKind::Ddc,
            enabled: true,
            override_percent: None,
            last_applied_percent: Some(25),
            last_applied_at_epoch_s: None,
            backoff_until_epoch_s: None,
            topology: None,
        })
        .collect();

    StatusResponse {
        config_path: String::from("/tmp/config.toml"),
        dry_run: false,
        tick_seconds: 30,
        daemon_alive: true,
        suspended: false,
        desktop_idle_dimmed: false,
        suspend_until_epoch_s: None,
        manual_override_active: false,
        per_monitor_override_until_epoch_s: None,
        global_override_percent: None,
        global_override_until_epoch_s: None,
        configured_monitors: monitor_count as u32,
        stateful_monitors: monitor_count as u32,
        weather: None,
        monitors,
        solar_elevation: Some(15.0),
        now_epoch_s: 0,
        sunrise_epoch_s: None,
        sunset_epoch_s: None,
        lunar_phase: None,
    }
}

pub(crate) fn named_monitor_config(
    logical_id: &str,
    display_name: &str,
    min_pct: u8,
    max_pct: u8,
) -> MonitorConfig {
    MonitorConfig {
        logical_id: logical_id.to_owned(),
        min_pct,
        max_pct,
        selector: MonitorSelector {
            model: Some(display_name.to_owned()),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn configure_monitor_fixture(model: &mut Model, monitors: Vec<MonitorConfig>) {
    model.config = Config::default();
    model.config.monitors = monitors;
    model.form = FormState::new(&model.config);
    model.selected_monitor = 0;
    model.selected_monitor_id = None;
}

pub(crate) fn key_event(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind,
        state: KeyEventState::empty(),
    }
}

pub(crate) fn press(model: &mut Model, code: KeyCode, modifiers: KeyModifiers) {
    update::update(
        model,
        Message::Key(key_event(code, modifiers, KeyEventKind::Press)),
    );
}

pub(crate) fn two_monitor_model() -> Model {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![
            named_monitor_config("mon-0", "Mi Monitor", 5, 60),
            named_monitor_config("mon-1", "LEN P24h-20", 8, 90),
        ],
    );
    model.status = Some(dummy_status(2));
    model.daemon_connection = DaemonConnection::Connected;
    model
}

pub(crate) fn find_in_buffer<'a>(buffer: &'a Buffer, text: &str) -> Option<(u16, u16, &'a Cell)> {
    for y in 0..buffer.area.height {
        let row = (0..buffer.area.width)
            .map(|x| buffer.get(x, y).symbol())
            .collect::<String>();
        if let Some(index) = row.find(text) {
            let x = row[..index].chars().count() as u16;
            return Some((x, y, buffer.get(x, y)));
        }
    }
    None
}

pub(crate) fn buffer_text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.get(x, y).symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn buffer_row(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer.get(x, y).symbol())
        .collect()
}

pub(crate) fn dummy_weather_status(
    enabled: bool,
    active: bool,
    stale: bool,
    cloud_cover: Option<u8>,
    multiplier: Option<f64>,
    forecast_count: usize,
) -> crate::ipc::WeatherStatus {
    let forecast = (0..forecast_count)
        .map(|i| crate::state::ForecastPoint {
            dt_epoch_s: 1_700_000_000 + (i as u64) * 3600 * 3,
            cloud_cover_percent: ((i * 15) % 100) as u8,
            temperature: 20.0 + (i as f32),
            ..Default::default()
        })
        .collect();

    crate::ipc::WeatherStatus {
        enabled,
        state: if active {
            crate::weather::WeatherState::Ready
        } else if stale {
            crate::weather::WeatherState::Stale
        } else {
            crate::weather::WeatherState::Loading
        },
        active,
        stale,
        provider: Some(String::from("openweather")),
        fetched_at_epoch_s: Some(1_700_000_000),
        valid_at_epoch_s: Some(1_700_000_000),
        source_kind: None,
        last_refresh_attempt_epoch_s: Some(1_700_000_000),
        next_refresh_at_epoch_s: Some(1_700_000_600),
        consecutive_failures: 0,
        last_error: None,
        cloud_cover_percent: cloud_cover,
        temperature: Some(21.5),
        condition: crate::weather::WeatherCondition::PartlyCloudy,
        condition_description: Some(String::from("broken clouds")),
        day_phase: Some(crate::weather::WeatherDayPhase::Day),
        forecast,
        multiplier,
        ..Default::default()
    }
}
