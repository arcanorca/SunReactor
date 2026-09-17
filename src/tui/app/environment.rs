use std::path::PathBuf;
use std::time::Duration;

use crate::config::{self as app_config, Config};
use crate::tui::form::FormState;
use crate::tui::model::{
    ActionState, DaemonConnection, InputMode, Model, MonitorDiscoveryState, Tab,
};
use crate::tui::worker::{spawn_ipc_worker, WorkerTarget};

/// Where a [`Model`] reads and writes configuration and reaches the daemon.
///
/// Production uses the user's session. Unit tests use an isolated environment
/// so a test run can never rewrite `~/.config/sunreactor/config.toml`, reload
/// or suspend the user's live daemon, or probe display hardware.
#[derive(Debug, Clone)]
pub(crate) struct ModelEnvironment {
    /// `None` loads `Config::default()` instead of reading a file.
    pub config_source: Option<PathBuf>,
    /// Destination for saved configuration; `None` writes the user file.
    pub config_save_path: Option<PathBuf>,
    pub worker: WorkerTarget,
}

impl ModelEnvironment {
    /// The interactive user session.
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn user_session() -> Result<Self, String> {
        let path = crate::paths::config_file().map_err(|error| error.to_string())?;
        Ok(Self {
            config_source: Some(path),
            config_save_path: None,
            worker: WorkerTarget {
                socket_path: None,
                hardware_discovery: true,
            },
        })
    }

    /// A hermetic environment: default config, saves into a private temporary
    /// directory, and a control socket path that no daemon listens on.
    #[cfg(test)]
    pub(crate) fn isolated() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sunreactor-tui-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::create_dir_all(&dir);
        Self {
            config_source: None,
            config_save_path: Some(dir.join("config.toml")),
            worker: WorkerTarget {
                socket_path: Some(dir.join("no-daemon.sock")),
                hardware_discovery: false,
            },
        }
    }
}

pub(crate) fn load_timezone(timezone_name: &str) -> Option<tz::TimeZone> {
    let path = std::path::Path::new("/usr/share/zoneinfo").join(timezone_name);
    std::fs::read(path)
        .ok()
        .and_then(|data| tz::TimeZone::from_tz_data(&data).ok())
        .or_else(|| tz::TimeZone::from_posix_tz(timezone_name).ok())
}

impl Model {
    #[must_use]
    pub fn new() -> Self {
        #[cfg(test)]
        let environment = ModelEnvironment::isolated();
        #[cfg(not(test))]
        let environment = ModelEnvironment::user_session().unwrap_or(ModelEnvironment {
            config_source: None,
            config_save_path: None,
            worker: WorkerTarget {
                socket_path: None,
                hardware_discovery: true,
            },
        });
        Self::with_environment(environment)
    }

    #[must_use]
    pub(crate) fn with_environment(environment: ModelEnvironment) -> Self {
        let (config, config_error) = match &environment.config_source {
            Some(path) => match app_config::load_from_path(path) {
                Ok(report) => (report.config, None),
                Err(error) => (Config::default(), Some(error.to_string())),
            },
            None => (Config::default(), None),
        };
        let form = FormState::new(&config);
        let (ipc_tx, ipc_rx) = spawn_ipc_worker(Duration::from_secs(2), environment.worker);
        let timezone_cache = load_timezone(&config.location.timezone);

        let mut monitor_list_state = ratatui::widgets::ListState::default();
        monitor_list_state.select(Some(0));

        let effects = config.tui.effects;
        let mut app = Self {
            should_quit: false,
            status: None,
            selected_monitor: 0,
            selected_monitor_id: None,
            ipc_tx,
            ipc_rx,
            daemon_connection: DaemonConnection::Unknown,
            monitor_discovery: MonitorDiscoveryState::NotRequested,
            config,
            config_error,
            active_tab: Tab::Monitors,

            show_help: false,
            help_scroll: 0,
            monitor_list_state,
            milestone_list_state: ratatui::widgets::ListState::default(),
            settings_scroll: 0,
            input_mode: InputMode::Normal,
            active_setting: 0,
            action_state: ActionState::Idle,
            next_command_id: 0,
            form,
            editing_form_snapshot: None,
            monitor_pane_focus: crate::tui::model::MonitorPaneFocus::List,
            monitor_control_index: 0,
            automation_focus: crate::tui::model::AutomationRegionFocus::Curve,
            selected_monitor_milestone: 0,
            monitor_milestones: Vec::new(),
            monitor_milestone_error: None,
            last_milestone_refresh_minute: None,
            monitor_milestones_dirty: false,
            config_dirty: false,
            last_config_mutation: None,
            active_modal: crate::tui::model::ActiveModal::None,
            motion: {
                let mut m = crate::tui::motion::UiMotionState::new();
                m.level = effects;
                m
            },
            last_weather_fetched_at: None,
            timezone_cache,
            cell_metrics: crate::tui::geometry::TerminalCellMetrics::default(),
            config_save_path: environment.config_save_path,
            monitor_targets_now: Vec::new(),
            monitor_curves: Vec::new(),
            curve_morph_from: None,
            preview_job: None,
            preview_generation: 0,
            tabs_focused: false,
            monitor_selector_horizontal: false,
        };
        app.refresh_monitor_milestones();
        app
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::ErrorCategory;
    use crate::tui::worker::IpcEvent;
    use std::time::Instant;

    #[test]
    fn test_model_in_tests_is_isolated_from_the_user_session() {
        let mut model = Model::new();
        let save_path = model
            .config_save_path
            .clone()
            .expect("unit tests never write the user's configuration file");
        assert!(save_path.starts_with(std::env::temp_dir()));
        assert!(
            model.config.monitors.is_empty(),
            "tests start from defaults"
        );

        // A save lands in the private directory and the reload request cannot
        // reach a daemon, so a test run can never reload or suspend a live one.
        assert!(model.save_config());
        assert!(save_path.exists());
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut outcome = None;
        while Instant::now() < deadline && outcome.is_none() {
            while let Ok(event) = model.ipc_rx.try_recv() {
                if let IpcEvent::CommandFailed { error_category, .. } = event {
                    outcome = Some(error_category);
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(outcome, Some(ErrorCategory::DaemonUnavailable));
    }
}
