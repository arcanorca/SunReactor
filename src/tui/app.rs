use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::config::{self as app_config, Config, MotionLevel};
use crate::ipc::Request;
use crate::policy::{self, MonitorMilestoneSchedule, PolicyContext};
use crate::solar::Location;

use super::form::FormState;
use super::globe::{rotation_duration, GlobeCenter};
use super::worker::{spawn_ipc_worker, IpcCommand, WorkerTarget};

use super::model::{
    settings_index, ActionKind, ActionState, ActiveInputKind, DaemonConnection, DaemonLifecycle,
    ErrorCategory, InputMode, Model, MonitorDiscoveryState, MonitorWorkspaceState,
    NextMilestoneInfo, OperationalMode, Tab,
};

/// Policy for initializing text input buffers when entering editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditBufferPolicy {
    /// Buffer retains current value and places cursor at the end.
    ExistingValue,
    /// Buffer is cleared for clean replacement (original preserved in snapshot for Esc).
    CleanReplacement,
}

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
            monitor_pane_focus: super::model::MonitorPaneFocus::List,
            monitor_control_index: 0,
            automation_focus: super::model::AutomationRegionFocus::Curve,
            selected_monitor_milestone: 0,
            monitor_milestones: Vec::new(),
            monitor_milestone_error: None,
            last_milestone_refresh_minute: None,
            monitor_milestones_dirty: false,
            config_dirty: false,
            last_config_mutation: None,
            active_modal: super::model::ActiveModal::None,
            motion: {
                let mut m = super::motion::UiMotionState::new();
                m.level = effects;
                m
            },
            last_weather_fetched_at: None,
            timezone_cache,
            cell_metrics: super::geometry::TerminalCellMetrics::default(),
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

    #[must_use]
    pub fn automation_field_count(&self) -> usize {
        self.form.automation_field_count()
    }

    pub fn tab_field_count(&self, tab: Tab) -> usize {
        match tab {
            Tab::Monitors => 0,
            Tab::Limits => self.automation_field_count(),
            Tab::Location => 4,
            Tab::Weather => 0,
            Tab::Settings => settings_index::COUNT,
        }
    }

    #[must_use]
    pub fn monitors_count(&self) -> usize {
        self.status.as_ref().map_or_else(
            || self.config.monitors.len(),
            |status| {
                if status.monitors.is_empty() {
                    self.config.monitors.len()
                } else {
                    status.monitors.len()
                }
            },
        )
    }

    fn monitor_logical_id_at(&self, index: usize) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|status| status.monitors.get(index))
            .map(|monitor| monitor.logical_id.as_str())
            .or_else(|| {
                self.config
                    .monitors
                    .get(index)
                    .map(|monitor| monitor.logical_id.as_str())
            })
    }

    fn current_monitor_index_for_id(&self, logical_id: &str) -> Option<usize> {
        if let Some(status) = self
            .status
            .as_ref()
            .filter(|status| !status.monitors.is_empty())
        {
            return status
                .monitors
                .iter()
                .position(|monitor| monitor.logical_id == logical_id);
        }

        self.config
            .monitors
            .iter()
            .position(|monitor| monitor.logical_id == logical_id)
    }

    fn select_monitor_index(&mut self, index: usize) {
        self.selected_monitor = index;
        self.selected_monitor_id = self.monitor_logical_id_at(index).map(str::to_owned);
        self.monitor_list_state.select(Some(index));
    }

    pub fn clamp_monitor_selection(&mut self) {
        let count = self.monitors_count();
        if count == 0 {
            self.selected_monitor = 0;
            self.monitor_list_state.select(None);
            return;
        }

        let selected_index = self
            .selected_monitor_id
            .as_deref()
            .and_then(|logical_id| self.current_monitor_index_for_id(logical_id))
            .unwrap_or_else(|| self.selected_monitor.min(count.saturating_sub(1)));
        self.select_monitor_index(selected_index);
    }

    #[must_use]
    pub(crate) fn monitor_workspace_state(&self) -> MonitorWorkspaceState {
        if self.daemon_connection == DaemonConnection::Disconnected {
            return MonitorWorkspaceState::DaemonUnavailable;
        }

        let Some(status) = self.status.as_ref() else {
            return MonitorWorkspaceState::Connecting;
        };
        if !status.daemon_alive {
            return MonitorWorkspaceState::DaemonUnavailable;
        }

        if !status.monitors.is_empty() {
            let enabled = status
                .monitors
                .iter()
                .filter(|monitor| monitor.enabled)
                .collect::<Vec<_>>();
            if !enabled.is_empty()
                && enabled
                    .iter()
                    .all(|monitor| monitor.topology.as_deref() == Some("temporarily_unavailable"))
            {
                return MonitorWorkspaceState::ConfiguredUnavailable;
            }
            return MonitorWorkspaceState::Ready;
        }

        if status.configured_monitors > 0 || !self.config.monitors.is_empty() {
            return MonitorWorkspaceState::ConfiguredUnavailable;
        }

        match &self.monitor_discovery {
            MonitorDiscoveryState::NotRequested | MonitorDiscoveryState::Loading => {
                MonitorWorkspaceState::Discovering
            }
            MonitorDiscoveryState::Complete(report) => {
                let compatible_count = report.viable_targets().len();
                if compatible_count == 0 {
                    MonitorWorkspaceState::NoCompatibleHardware
                } else {
                    MonitorWorkspaceState::DiscoveredButUnconfigured {
                        compatible_count,
                        importable_count: report.importable_monitor_configs().len(),
                        incomplete_ddc_probe: report.has_incomplete_ddc_probe(),
                    }
                }
            }
        }
    }

    #[must_use]
    pub(crate) fn monitor_discovery_report(&self) -> Option<&crate::discovery::DiscoveryReport> {
        match &self.monitor_discovery {
            MonitorDiscoveryState::Complete(report) => Some(report),
            MonitorDiscoveryState::NotRequested | MonitorDiscoveryState::Loading => None,
        }
    }

    /// Start one background probe only after the daemon has proven that its
    /// active configuration is empty. Discovery is never inferred from a
    /// failed daemon connection or repeated on every status poll.
    pub fn request_monitor_discovery_if_needed(&mut self) {
        let daemon_has_empty_config = self.status.as_ref().is_some_and(|status| {
            status.daemon_alive && status.configured_monitors == 0 && status.monitors.is_empty()
        });
        if !daemon_has_empty_config
            || !self.config.monitors.is_empty()
            || !matches!(self.monitor_discovery, MonitorDiscoveryState::NotRequested)
        {
            return;
        }

        if self.ipc_tx.try_send(IpcCommand::DiscoverMonitors).is_ok() {
            self.monitor_discovery = MonitorDiscoveryState::Loading;
        }
    }

    pub fn retry_monitor_discovery(&mut self) {
        if matches!(self.monitor_discovery, MonitorDiscoveryState::Loading) {
            return;
        }
        self.monitor_discovery = MonitorDiscoveryState::NotRequested;
        self.request_monitor_discovery_if_needed();
    }

    /// Persist only discovery candidates that already passed the same unique
    /// selector and topology-retargeting checks as generated config snippets.
    pub fn import_discovered_monitors(&mut self) {
        if !self.config.monitors.is_empty() {
            return;
        }

        let Some(report) = self.monitor_discovery_report() else {
            self.request_monitor_discovery_if_needed();
            return;
        };
        if report.has_incomplete_ddc_probe() {
            self.retry_monitor_discovery();
            return;
        }
        let monitors = report.importable_monitor_configs();
        if monitors.is_empty() {
            self.retry_monitor_discovery();
            return;
        }

        let count = monitors.len();
        let selected_monitor_id = monitors.first().map(|monitor| monitor.logical_id.clone());
        let mut candidate = self.config.clone();
        candidate.monitors = monitors;
        if let Err(error) = candidate.validate() {
            self.config_error = Some(error.to_string());
            self.action_state = ActionState::Error {
                action: ActionKind::SaveConfig,
                category: ErrorCategory::Validation,
                message: format!("Cannot add discovered monitors: {error}"),
                completed_at: Instant::now(),
            };
            return;
        }

        if self.commit_config(candidate, format!("Adding {count} compatible monitor(s)…")) {
            self.selected_monitor = 0;
            self.selected_monitor_id = selected_monitor_id;
            self.clamp_monitor_selection();
        }
    }

    fn selected_config_monitor_index(&self) -> Option<usize> {
        self.selected_monitor_logical_id()
            .and_then(|logical_id| {
                self.config
                    .monitors
                    .iter()
                    .position(|monitor| monitor.logical_id == logical_id)
            })
            .or_else(|| {
                (self.selected_monitor < self.config.monitors.len())
                    .then_some(self.selected_monitor)
            })
    }

    pub fn switch_to_tab(&mut self, tab: Tab) {
        if self.active_tab == tab {
            return;
        }
        if self.config_dirty && !self.save_config() {
            return;
        }
        self.motion
            .trigger_tab_slide(self.active_tab.index(), tab.index(), Instant::now());
        self.active_tab = tab;
        self.active_setting = 0;
        self.settings_scroll = 0;
        if matches!(tab, Tab::Monitors) {
            self.monitor_pane_focus = super::model::MonitorPaneFocus::List;
        }
        if matches!(tab, Tab::Limits) {
            // With several monitors the choice of monitor comes first, so
            // Automation opens on the monitor chips.
            self.automation_focus = if self.monitors_count() > 1 {
                super::model::AutomationRegionFocus::Selector
            } else {
                super::model::AutomationRegionFocus::Curve
            };
        }
        if matches!(tab, Tab::Location) {
            // Full effects bring the globe round to the configured location;
            // the settled rotation hands over to a short acquisition pulse.
            let target = GlobeCenter::new(
                self.config.location.longitude,
                self.config.location.latitude,
            );
            let from = GlobeCenter::new(target.lon_deg - 75.0, target.lat_deg * 0.4);
            self.motion.trigger(
                super::motion::TransientKind::GlobeRotation {
                    from_lon: from.lon_deg,
                    from_lat: from.lat_deg,
                    to_lon: target.lon_deg,
                    to_lat: target.lat_deg,
                },
                Instant::now(),
                rotation_duration(from, target),
            );
        }
    }

    /// Whether keyboard focus is inside the workspace rather than on the tab bar.
    #[must_use]
    pub(crate) fn workspace_focused(&self) -> bool {
        !self.tabs_focused
    }

    pub fn next_tab(&mut self) {
        self.switch_to_tab(self.active_tab.next());
    }

    pub fn previous_tab(&mut self) {
        self.switch_to_tab(self.active_tab.previous());
    }

    pub fn move_selection_down(&mut self) {
        match self.active_tab {
            Tab::Monitors => {
                let count = self.monitors_count();
                if count > 0 && self.selected_monitor < count.saturating_sub(1) {
                    self.selected_monitor_milestone = 0;
                    self.select_monitor_index(self.selected_monitor + 1);
                }
            }
            tab => {
                let max_setting = self.tab_field_count(tab).saturating_sub(1);
                if self.active_setting < max_setting {
                    self.active_setting += 1;
                }
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        match self.active_tab {
            Tab::Monitors => {
                if self.selected_monitor > 0 {
                    self.selected_monitor_milestone = 0;
                    self.select_monitor_index(self.selected_monitor - 1);
                }
            }
            _ if self.active_setting > 0 => {
                self.active_setting -= 1;
            }
            _ => {}
        }
    }

    pub fn page_down(&mut self) {
        match self.active_tab {
            Tab::Monitors => {
                let count = self.monitors_count();
                if count > 0 {
                    self.select_monitor_index(
                        (self.selected_monitor + 4).min(count.saturating_sub(1)),
                    );
                }
            }
            tab => {
                let max_setting = self.tab_field_count(tab).saturating_sub(1);
                self.active_setting = (self.active_setting + 4).min(max_setting);
            }
        }
    }

    pub fn page_up(&mut self) {
        match self.active_tab {
            Tab::Monitors => {
                if self.monitors_count() > 0 {
                    self.select_monitor_index(self.selected_monitor.saturating_sub(4));
                }
            }
            _ => {
                self.active_setting = self.active_setting.saturating_sub(4);
            }
        }
    }

    pub fn move_to_first(&mut self) {
        match self.active_tab {
            Tab::Monitors => {
                if self.monitors_count() > 0 {
                    self.select_monitor_index(0);
                }
            }
            _ => {
                self.active_setting = 0;
            }
        }
    }

    pub fn move_to_last(&mut self) {
        match self.active_tab {
            Tab::Monitors => {
                let count = self.monitors_count();
                if count > 0 {
                    self.select_monitor_index(count.saturating_sub(1));
                }
            }
            tab => {
                self.active_setting = self.tab_field_count(tab).saturating_sub(1);
            }
        }
    }

    pub fn scroll_help_up(&mut self, lines: usize) {
        self.help_scroll = self.help_scroll.saturating_sub(lines);
    }

    pub fn scroll_help_down(&mut self, lines: usize, max_scroll: usize) {
        self.help_scroll = (self.help_scroll + lines).min(max_scroll);
    }

    pub fn scroll_help_home(&mut self) {
        self.help_scroll = 0;
    }

    pub fn scroll_help_end(&mut self, max_scroll: usize) {
        self.help_scroll = max_scroll;
    }

    pub fn update_cell_metrics(&mut self) {
        self.cell_metrics = super::geometry::detect_terminal_cell_metrics();
    }

    pub fn handle_resize(&mut self, _cols: u16, _rows: u16) {
        self.update_cell_metrics();
        self.clamp_monitor_selection();
        let max_setting = self.tab_field_count(self.active_tab).saturating_sub(1);
        if self.active_setting > max_setting {
            self.active_setting = max_setting;
        }
    }

    pub fn toggle_active_setting(&mut self) {
        if self.active_tab == Tab::Settings {
            match self.active_setting {
                settings_index::THEME => {
                    let mut state = ratatui::widgets::ListState::default();
                    let current_index = crate::tui::theme::Theme::ALL
                        .iter()
                        .position(|t| *t == self.config.tui.theme)
                        .unwrap_or(0);
                    state.select(Some(current_index));
                    self.active_modal =
                        super::model::ActiveModal::ThemeSelect(state, self.config.tui.theme);
                }
                settings_index::EFFECTS => {
                    self.config.tui.effects = self.config.tui.effects.next();
                    self.motion.level = self.config.tui.effects;
                    if self.motion.level == MotionLevel::Off {
                        self.motion.active_transient = None;
                    }
                    self.mark_settings_config_dirty();
                }
                settings_index::SHOW_LOGO => {
                    self.config.tui.show_logo = !self.config.tui.show_logo;
                    self.mark_settings_config_dirty();
                }
                settings_index::TIME_FORMAT => {
                    self.config.tui.use_12h_time = !self.config.tui.use_12h_time;
                    self.mark_settings_config_dirty();
                }
                settings_index::TEMPERATURE_UNIT => {
                    self.config.tui.temperature_unit = match self.config.tui.temperature_unit {
                        crate::config::TemperatureUnit::Celsius => {
                            crate::config::TemperatureUnit::Fahrenheit
                        }
                        crate::config::TemperatureUnit::Fahrenheit => {
                            crate::config::TemperatureUnit::Celsius
                        }
                    };
                    self.mark_settings_config_dirty();
                }
                settings_index::WEATHER_ENABLED => {
                    self.config.weather.enabled = !self.config.weather.enabled;
                    self.mark_settings_config_dirty();
                }
                _ => {}
            }
        }
    }

    pub fn cycle_active_setting_backward(&mut self) {
        if self.active_tab == Tab::Settings {
            match self.active_setting {
                settings_index::EFFECTS => {
                    self.config.tui.effects = self.config.tui.effects.previous();
                    self.motion.level = self.config.tui.effects;
                    if self.motion.level == MotionLevel::Off {
                        self.motion.active_transient = None;
                    }
                    self.mark_settings_config_dirty();
                }
                settings_index::SHOW_LOGO => {
                    self.config.tui.show_logo = !self.config.tui.show_logo;
                    self.mark_settings_config_dirty();
                }
                settings_index::TIME_FORMAT => {
                    self.config.tui.use_12h_time = !self.config.tui.use_12h_time;
                    self.mark_settings_config_dirty();
                }
                settings_index::TEMPERATURE_UNIT => {
                    self.config.tui.temperature_unit = match self.config.tui.temperature_unit {
                        crate::config::TemperatureUnit::Celsius => {
                            crate::config::TemperatureUnit::Fahrenheit
                        }
                        crate::config::TemperatureUnit::Fahrenheit => {
                            crate::config::TemperatureUnit::Celsius
                        }
                    };
                    self.mark_settings_config_dirty();
                }
                settings_index::WEATHER_ENABLED => {
                    self.config.weather.enabled = !self.config.weather.enabled;
                    self.mark_settings_config_dirty();
                }
                _ => {}
            }
        }
    }

    fn mark_settings_config_dirty(&mut self) {
        self.config_dirty = true;
        self.last_config_mutation = Some(Instant::now());
    }

    pub fn theme_modal_down(&mut self) {
        if let super::model::ActiveModal::ThemeSelect(ref mut state, _) = self.active_modal {
            let i = match state.selected() {
                Some(i) => {
                    if i >= crate::tui::theme::Theme::ALL.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            state.select(Some(i));
            self.config.tui.theme = crate::tui::theme::Theme::ALL[i];
        }
    }

    pub fn theme_modal_up(&mut self) {
        if let super::model::ActiveModal::ThemeSelect(ref mut state, _) = self.active_modal {
            let i = match state.selected() {
                Some(i) => {
                    if i == 0 {
                        crate::tui::theme::Theme::ALL.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            state.select(Some(i));
            self.config.tui.theme = crate::tui::theme::Theme::ALL[i];
        }
    }

    pub fn theme_modal_confirm(&mut self) {
        if let super::model::ActiveModal::ThemeSelect(_, _) = self.active_modal {
            self.active_modal = super::model::ActiveModal::None;
            self.save_config();
        }
    }

    pub fn theme_modal_cancel(&mut self) {
        if let super::model::ActiveModal::ThemeSelect(_, original_theme) = self.active_modal {
            self.active_modal = super::model::ActiveModal::None;
            self.config.tui.theme = original_theme;
        }
    }

    /// Determines the input initialization policy for the currently focused field.
    #[must_use]
    pub fn edit_buffer_policy(&self) -> EditBufferPolicy {
        if (matches!(self.active_tab, Tab::Location) && self.active_setting == 0)
            || (matches!(self.active_tab, Tab::Settings)
                && self.active_setting == settings_index::WEATHER_API_KEY)
        {
            EditBufferPolicy::CleanReplacement
        } else {
            EditBufferPolicy::ExistingValue
        }
    }

    pub fn start_editing(&mut self) {
        if matches!(self.active_tab, Tab::Monitors)
            && !matches!(
                self.monitor_pane_focus,
                super::model::MonitorPaneFocus::Detail
            )
        {
            return;
        }
        self.editing_form_snapshot = Some(self.form.clone());
        self.input_mode = InputMode::Editing;

        match self.edit_buffer_policy() {
            EditBufferPolicy::CleanReplacement => {
                if matches!(self.active_tab, Tab::Location) && self.active_setting == 0 {
                    self.form.city_search_input = tui_input::Input::default();
                    self.form.city_search_results.clear();
                    self.form.city_search_selected_index = 0;
                } else if matches!(self.active_tab, Tab::Settings)
                    && self.active_setting == settings_index::WEATHER_API_KEY
                {
                    self.form.api_key_input = tui_input::Input::default();
                }
            }
            EditBufferPolicy::ExistingValue => {
                if let Some(input) = self.active_input_mut() {
                    let len = input.value().chars().count();
                    input.handle(tui_input::InputRequest::SetCursor(len));
                }
            }
        }
    }

    pub fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal;
        self.editing_form_snapshot = None;
    }

    pub fn cancel_editing(&mut self) {
        if let Some(snapshot) = self.editing_form_snapshot.take() {
            self.form = snapshot;
        }
        self.config_error = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn handle_input_request(&mut self, req: tui_input::InputRequest) -> bool {
        if let Some(input) = self.active_input_mut() {
            input.handle(req).is_some()
        } else {
            false
        }
    }

    pub fn insert_char_to_active_input(&mut self, c: char) {
        self.handle_input_request(tui_input::InputRequest::InsertChar(c));
    }

    pub fn backspace_active_input(&mut self) {
        self.handle_input_request(tui_input::InputRequest::DeletePrevChar);
    }

    pub fn delete_active_input(&mut self) {
        self.handle_input_request(tui_input::InputRequest::DeleteNextChar);
    }

    pub fn move_cursor_left(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToPrevChar);
    }

    pub fn move_cursor_right(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToNextChar);
    }

    pub fn move_cursor_start(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToStart);
    }

    pub fn move_cursor_end(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToEnd);
    }

    pub fn delete_prev_word_active_input(&mut self) {
        self.handle_input_request(tui_input::InputRequest::DeletePrevWord);
    }

    pub fn move_cursor_prev_word(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToPrevWord);
    }

    pub fn move_cursor_next_word(&mut self) {
        self.handle_input_request(tui_input::InputRequest::GoToNextWord);
    }

    #[must_use]
    pub fn active_input_kind(&self) -> Option<ActiveInputKind> {
        if matches!(self.active_tab, Tab::Monitors) {
            if matches!(
                self.monitor_pane_focus,
                super::model::MonitorPaneFocus::Detail
            ) {
                match self.monitor_control_index {
                    0 | 1 => Some(ActiveInputKind::Integer),
                    _ => None,
                }
            } else {
                None
            }
        } else if matches!(self.active_tab, Tab::Limits) {
            if matches!(
                self.automation_focus,
                super::model::AutomationRegionFocus::Curve
            ) {
                Some(ActiveInputKind::Decimal)
            } else {
                None
            }
        } else {
            self.form
                .active_input_kind(self.active_tab, self.active_setting)
        }
    }

    #[must_use]
    pub fn active_input_ref(&self) -> Option<&tui_input::Input> {
        if matches!(self.active_tab, Tab::Monitors) {
            if matches!(
                self.monitor_pane_focus,
                super::model::MonitorPaneFocus::Detail
            ) {
                let pair = self
                    .selected_config_monitor_index()
                    .and_then(|index| self.form.monitor_inputs.get(index));
                match self.monitor_control_index {
                    0 => pair.map(|p| &p.0),
                    1 => pair.map(|p| &p.1),
                    _ => None,
                }
            } else {
                None
            }
        } else if matches!(self.active_tab, Tab::Limits) {
            if matches!(
                self.automation_focus,
                super::model::AutomationRegionFocus::Curve
            ) {
                self.selected_config_monitor_index()
                    .and_then(|index| self.form.monitor_curve_inputs.get(index))
            } else {
                None
            }
        } else {
            self.form
                .active_input_ref(self.active_tab, self.active_setting)
        }
    }

    pub fn active_input_mut(&mut self) -> Option<&mut tui_input::Input> {
        if matches!(self.active_tab, Tab::Monitors) {
            if matches!(
                self.monitor_pane_focus,
                super::model::MonitorPaneFocus::Detail
            ) {
                let idx = self.monitor_control_index;
                let monitor_index = self.selected_config_monitor_index()?;
                let pair = self.form.monitor_inputs.get_mut(monitor_index);
                match idx {
                    0 => pair.map(|p| &mut p.0),
                    1 => pair.map(|p| &mut p.1),
                    _ => None,
                }
            } else {
                None
            }
        } else if matches!(self.active_tab, Tab::Limits) {
            if matches!(
                self.automation_focus,
                super::model::AutomationRegionFocus::Curve
            ) {
                let monitor_index = self.selected_config_monitor_index()?;
                self.form.monitor_curve_inputs.get_mut(monitor_index)
            } else {
                None
            }
        } else {
            self.form
                .active_input_mut(self.active_tab, self.active_setting)
        }
    }

    pub fn select_next_monitor(&mut self) {
        let count = self.monitors_count();
        if count > 1 {
            self.selected_monitor_milestone = 0;
            self.select_monitor_index((self.selected_monitor + 1) % count);
            self.refresh_monitor_milestones_if_empty();
        }
    }

    pub fn select_previous_monitor(&mut self) {
        let count = self.monitors_count();
        if count > 1 {
            let selected_index = if self.selected_monitor == 0 {
                count.saturating_sub(1)
            } else {
                self.selected_monitor - 1
            };
            self.selected_monitor_milestone = 0;
            self.select_monitor_index(selected_index);
            self.refresh_monitor_milestones_if_empty();
        }
    }

    pub fn step_monitor_range(&mut self, delta: i8) {
        let Some(monitor_idx) = self.selected_config_monitor_index() else {
            return;
        };
        let is_min = self.monitor_control_index == 0;
        let changed_value = self
            .config
            .monitors
            .get_mut(monitor_idx)
            .and_then(|monitor| {
                let old_value = if is_min {
                    monitor.min_pct
                } else {
                    monitor.max_pct
                };
                let new_value = if is_min {
                    (i16::from(monitor.min_pct) + i16::from(delta))
                        .clamp(0, i16::from(monitor.max_pct)) as u8
                } else {
                    (i16::from(monitor.max_pct) + i16::from(delta))
                        .clamp(i16::from(monitor.min_pct), 100) as u8
                };
                if new_value == old_value {
                    return None;
                }
                if is_min {
                    monitor.min_pct = new_value;
                } else {
                    monitor.max_pct = new_value;
                }
                Some(new_value)
            });

        if let Some(new_value) = changed_value {
            if let Some(pair) = self.form.monitor_inputs.get_mut(monitor_idx) {
                let input = tui_input::Input::default().with_value(new_value.to_string());
                if is_min {
                    pair.0 = input;
                } else {
                    pair.1 = input;
                }
            }
            self.mark_monitor_policy_dirty();
            self.motion.trigger(
                crate::tui::motion::TransientKind::RangeCommit { min: is_min },
                Instant::now(),
                Duration::from_millis(250),
            );
        }
    }

    pub fn step_monitor_curve(&mut self, delta: f64) {
        let Some(monitor_idx) = self.selected_config_monitor_index() else {
            return;
        };
        let changed_value = self
            .config
            .monitors
            .get_mut(monitor_idx)
            .and_then(|monitor| {
                let new_value = ((monitor.transition_gamma + delta) * 20.0).round() / 20.0;
                let new_value = new_value.clamp(0.05, crate::config::MAX_TRANSITION_GAMMA);
                if (new_value - monitor.transition_gamma).abs() <= 1e-4 {
                    return None;
                }
                monitor.transition_gamma = new_value;
                Some(new_value)
            });

        if let Some(new_value) = changed_value {
            if let Some(input) = self.form.monitor_curve_inputs.get_mut(monitor_idx) {
                *input = tui_input::Input::default().with_value(format!("{new_value:.2}"));
            }
            self.mark_monitor_policy_dirty();
        }
    }

    fn mark_monitor_policy_dirty(&mut self) {
        self.config_dirty = true;
        self.last_config_mutation = Some(Instant::now());
        self.monitor_milestones_dirty = true;
    }

    pub fn send_command(&mut self, action: ActionKind, request: Request) -> u64 {
        self.next_command_id = self.next_command_id.wrapping_add(1);
        let command_id = self.next_command_id;
        let _ = self.ipc_tx.try_send(IpcCommand::Send {
            command_id,
            action,
            request,
        });
        command_id
    }

    pub fn check_debounced_save(&mut self) {
        if self.config_dirty {
            if let Some(mutation_time) = self.last_config_mutation {
                if mutation_time.elapsed() >= Duration::from_millis(400) {
                    self.save_config();
                }
            }
        }
    }

    pub fn check_action_state_expiration(&mut self) {
        let now = Instant::now();
        match &self.action_state {
            ActionState::Success { completed_at, .. } => {
                if now.duration_since(*completed_at) >= Duration::from_secs(3) {
                    self.action_state = ActionState::Idle;
                }
            }
            ActionState::Warning { completed_at, .. } => {
                if now.duration_since(*completed_at) >= Duration::from_secs(6) {
                    self.action_state = ActionState::Idle;
                }
            }
            ActionState::Error { completed_at, .. } => {
                if now.duration_since(*completed_at) >= Duration::from_secs(6) {
                    self.action_state = ActionState::Idle;
                }
            }
            ActionState::Pending {
                command_id,
                started_at,
                action,
                ..
            } => {
                if now.duration_since(*started_at) >= Duration::from_secs(8) {
                    if *action == ActionKind::SaveConfig {
                        tracing::warn!(
                            command_id = *command_id,
                            category = ?ErrorCategory::Timeout,
                            "daemon_reload_failed"
                        );
                        tracing::warn!(
                            command_id = *command_id,
                            category = ?ErrorCategory::Timeout,
                            "config_saved_daemon_unsynced"
                        );
                        self.action_state = ActionState::Warning {
                            action: *action,
                            category: ErrorCategory::Timeout,
                            message: String::from("Saved — daemon did not respond"),
                            completed_at: now,
                        };
                    } else {
                        self.action_state = ActionState::Error {
                            action: *action,
                            category: ErrorCategory::Timeout,
                            message: format!(
                                "{}: daemon did not respond (timeout)",
                                action.label()
                            ),
                            completed_at: now,
                        };
                    }
                }
            }
            ActionState::Idle => {}
        }
    }

    pub fn save_config(&mut self) -> bool {
        if let Err(error) = self.form.validate_values(&self.config) {
            tracing::warn!(error = %error, "validation_failed");
            self.config_error = Some(error.clone());
            self.config_dirty = true;
            self.last_config_mutation = None;
            self.action_state = ActionState::Error {
                action: ActionKind::SaveConfig,
                category: ErrorCategory::Validation,
                message: format!("Invalid value: {error}"),
                completed_at: Instant::now(),
            };
            return false;
        }

        let mut candidate = self.config.clone();
        self.form.apply_to_config(&mut candidate);

        self.commit_config(candidate, String::from("Saving configuration…"))
    }

    fn commit_config(&mut self, candidate: Config, description: String) -> bool {
        let previous_center = GlobeCenter::new(
            self.config.location.longitude,
            self.config.location.latitude,
        );
        let next_center =
            GlobeCenter::new(candidate.location.longitude, candidate.location.latitude);
        let location_changed = (previous_center.lon_deg - next_center.lon_deg).abs() > 1e-9
            || (previous_center.lat_deg - next_center.lat_deg).abs() > 1e-9;
        tracing::info!("config_write_started");
        let saved = match &self.config_save_path {
            Some(path) => app_config::save_to_path(&candidate, path),
            None => app_config::save(&candidate),
        };
        match saved {
            Ok(_) => {
                tracing::info!("config_write_succeeded");
                self.config = candidate;
                if location_changed {
                    self.motion.trigger(
                        super::motion::TransientKind::GlobeRotation {
                            from_lon: previous_center.lon_deg,
                            from_lat: previous_center.lat_deg,
                            to_lon: next_center.lon_deg,
                            to_lat: next_center.lat_deg,
                        },
                        Instant::now(),
                        rotation_duration(previous_center, next_center),
                    );
                }
                self.timezone_cache = load_timezone(&self.config.location.timezone);
                self.config_error = None;
                self.config_dirty = false;
                self.last_config_mutation = None;

                self.form.refresh_from_config(&self.config);
                self.request_preview_refresh();
                self.clamp_monitor_selection();
                let command_id = self.send_command(ActionKind::SaveConfig, Request::ReloadConfig);
                tracing::info!(command_id, "daemon_reload_requested");
                self.action_state = ActionState::Pending {
                    command_id,
                    action: ActionKind::SaveConfig,
                    description,
                    started_at: Instant::now(),
                };
                true
            }
            Err(error) => {
                tracing::error!(error = %error, "config_write_failed");
                self.config_error = Some(error.to_string());
                self.last_config_mutation = None;
                self.config_dirty = true;
                self.action_state = ActionState::Error {
                    action: ActionKind::SaveConfig,
                    category: ErrorCategory::ConfigWrite,
                    message: format!("Save failed: {error}"),
                    completed_at: Instant::now(),
                };
                false
            }
        }
    }

    pub fn suspend_writes(&mut self) {
        if matches!(
            self.action_state,
            ActionState::Pending {
                action: ActionKind::Suspend,
                ..
            }
        ) {
            return;
        }

        match self.form.suspend_duration_minutes() {
            Ok(minutes) => {
                let desc = match minutes {
                    Some(m) => format!("Suspending writes for {m}m…"),
                    None => String::from("Suspending writes until resume…"),
                };
                let command_id =
                    self.send_command(ActionKind::Suspend, Request::Suspend { minutes });
                self.action_state = ActionState::Pending {
                    command_id,
                    action: ActionKind::Suspend,
                    description: desc,
                    started_at: Instant::now(),
                };
            }
            Err(error) => {
                self.action_state = ActionState::Error {
                    action: ActionKind::Suspend,
                    category: ErrorCategory::Validation,
                    message: format!("Invalid value: {error}"),
                    completed_at: Instant::now(),
                };
            }
        }
    }

    pub fn resume_writes(&mut self) {
        if matches!(
            self.action_state,
            ActionState::Pending {
                action: ActionKind::Resume,
                ..
            }
        ) {
            return;
        }

        let command_id = self.send_command(ActionKind::Resume, Request::Resume);
        self.action_state = ActionState::Pending {
            command_id,
            action: ActionKind::Resume,
            description: String::from("Resuming daemon writes…"),
            started_at: Instant::now(),
        };
    }

    pub fn retry_weather(&mut self) {
        if matches!(
            self.action_state,
            ActionState::Pending {
                action: ActionKind::RefreshWeather,
                ..
            }
        ) {
            return;
        }

        let command_id = self.send_command(ActionKind::RefreshWeather, Request::RefreshWeather);
        self.action_state = ActionState::Pending {
            command_id,
            action: ActionKind::RefreshWeather,
            description: String::from("Refreshing weather…"),
            started_at: Instant::now(),
        };
    }

    #[must_use]
    pub fn selected_monitor_schedule(&self) -> Option<&MonitorMilestoneSchedule> {
        let logical_id = self.selected_monitor_logical_id()?;
        self.monitor_milestones
            .iter()
            .find(|schedule| schedule.logical_id == logical_id)
    }

    #[must_use]
    pub fn selected_monitor_logical_id(&self) -> Option<&str> {
        self.selected_monitor_id
            .as_deref()
            .or_else(|| self.monitor_logical_id_at(self.selected_monitor))
    }

    #[must_use]
    pub fn daemon_lifecycle(&self) -> DaemonLifecycle {
        if self.daemon_connection == DaemonConnection::Disconnected {
            return DaemonLifecycle::Unreachable;
        }
        match &self.status {
            Some(status) if status.suspended => DaemonLifecycle::Suspended,
            Some(status) if status.desktop_idle_dimmed => DaemonLifecycle::IdleDimmed,
            Some(status) if status.daemon_alive => DaemonLifecycle::Active,
            _ => DaemonLifecycle::Unreachable,
        }
    }

    #[must_use]
    pub fn operational_mode(&self) -> OperationalMode {
        if self.daemon_connection == DaemonConnection::Disconnected {
            return OperationalMode::Offline;
        }
        match &self.status {
            Some(status) if !status.daemon_alive => OperationalMode::Offline,
            Some(status) if status.suspended => OperationalMode::Suspended,
            Some(status) if status.desktop_idle_dimmed => OperationalMode::IdleDimmed,
            Some(status) if status.manual_override_active => OperationalMode::Override,
            Some(_) => OperationalMode::Automatic,
            None => OperationalMode::Offline,
        }
    }

    #[must_use]
    pub fn next_automation_milestone(
        &self,
        now_local: &chrono::DateTime<chrono::FixedOffset>,
    ) -> Option<NextMilestoneInfo> {
        let schedule = self
            .selected_monitor_schedule()
            .or_else(|| self.monitor_milestones.first())?;

        let (next, is_tomorrow) = if let Some(future) = schedule
            .milestones
            .iter()
            .find(|m| m.adjusted_time_local > *now_local)
        {
            (future, false)
        } else {
            (schedule.milestones.first()?, true)
        };

        let time_str = if self.config.tui.use_12h_time {
            next.adjusted_time_local.format("%I:%M %p").to_string()
        } else {
            next.adjusted_time_local.format("%H:%M").to_string()
        };

        Some(NextMilestoneInfo {
            milestone_label: next.milestone.label(),
            target_percent: next.target_percent,
            time_str,
            minutes_offset: next.minutes_offset,
            is_tomorrow,
        })
    }

    #[must_use]
    pub fn current_local_time(&self) -> chrono::DateTime<chrono::FixedOffset> {
        self.local_time_at(chrono::Utc::now())
    }

    #[must_use]
    pub(crate) fn local_time_at_epoch(
        &self,
        epoch_s: u64,
    ) -> Option<chrono::DateTime<chrono::FixedOffset>> {
        let epoch_s = i64::try_from(epoch_s).ok()?;
        chrono::DateTime::from_timestamp(epoch_s, 0).map(|utc| self.local_time_at(utc))
    }

    fn local_time_at(
        &self,
        now_utc: chrono::DateTime<chrono::Utc>,
    ) -> chrono::DateTime<chrono::FixedOffset> {
        let offset = self
            .timezone_cache
            .as_ref()
            .and_then(|timezone| {
                timezone
                    .find_local_time_type(now_utc.timestamp())
                    .map(tz::LocalTimeType::ut_offset)
                    .ok()
            })
            .and_then(chrono::FixedOffset::east_opt);

        if let Some(offset) = offset {
            now_utc.with_timezone(&offset)
        } else {
            let local = chrono::Local::now();
            let offset_sec = local.offset().local_minus_utc();
            let fixed = chrono::FixedOffset::east_opt(offset_sec)
                .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).expect("UTC offset is valid"));
            local.with_timezone(&fixed)
        }
    }

    pub fn refresh_monitor_milestones_if_needed(&mut self) {
        // Keyboard range/curve adjustments are coalesced. Rebuilding the daily
        // solar schedule here would put expensive work back on a key-repeat path.
        if self.monitor_milestones_dirty {
            return;
        }
        let minute = Utc::now().timestamp() / 60;
        if self.last_milestone_refresh_minute == Some(minute) || self.preview_job.is_some() {
            return;
        }
        self.request_preview_refresh();
    }

    fn refresh_monitor_milestones_if_empty(&mut self) {
        if self.monitor_milestones.is_empty() && !self.monitor_milestones_dirty {
            self.refresh_monitor_milestones();
        }
    }

    #[must_use]
    pub fn monitor_policy_preview_pending(&self) -> bool {
        self.monitor_milestones_dirty
    }

    /// Lightweight offset-only update: skips the expensive solar simulation
    /// and only recalculates adjusted times from existing base times + config offsets.
    /// Use after `adjust_selected_monitor_milestone` or `reset_selected_monitor_milestone`.
    pub fn reapply_milestone_offsets(&mut self) {
        for schedule in &mut self.monitor_milestones {
            let Some(monitor) = self
                .config
                .monitors
                .iter()
                .find(|m| m.logical_id == schedule.logical_id)
            else {
                continue;
            };
            for milestone_entry in &mut schedule.milestones {
                let offset = monitor
                    .milestone_adjustments
                    .iter()
                    .find(|a| a.milestone == milestone_entry.milestone)
                    .map_or(0i16, |a| a.minutes_offset);
                milestone_entry.minutes_offset = offset;
                milestone_entry.adjusted_time_local =
                    milestone_entry.base_time_local + chrono::Duration::minutes(i64::from(offset));
            }
            // Re-enforce monotonicity: each adjusted time must be >= previous + 1 min
            for i in 1..schedule.milestones.len() {
                let minimum =
                    schedule.milestones[i - 1].adjusted_time_local + chrono::Duration::minutes(1);
                if schedule.milestones[i].adjusted_time_local < minimum {
                    schedule.milestones[i].adjusted_time_local = minimum;
                }
            }
            // Backward pass: each adjusted time must be <= next - 1 min
            for i in (0..schedule.milestones.len().saturating_sub(1)).rev() {
                let maximum =
                    schedule.milestones[i + 1].adjusted_time_local - chrono::Duration::minutes(1);
                if schedule.milestones[i].adjusted_time_local > maximum {
                    schedule.milestones[i].adjusted_time_local = maximum;
                }
            }
        }
    }

    /// Recomputes the schedule and previews synchronously. Used at startup and
    /// by tests; interactive paths use [`Self::request_preview_refresh`].
    pub fn refresh_monitor_milestones(&mut self) {
        let job = self.preview_inputs();
        self.preview_job = None;
        let outcome = compute_preview_outcome(&job);
        self.apply_preview_outcome(outcome);
    }

    /// Starts a background recomputation. A newer request supersedes an
    /// older one; stale results are discarded when they arrive.
    pub fn request_preview_refresh(&mut self) {
        let job = self.preview_inputs();
        let generation = job.generation;
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name(String::from("sunreactor-preview"))
            .spawn(move || {
                let _ = tx.send(compute_preview_outcome(&job));
            });
        match spawned {
            Ok(_) => self.preview_job = Some((generation, rx)),
            Err(_) => self.refresh_monitor_milestones(),
        }
    }

    /// Applies a finished background recomputation, if any.
    pub fn poll_preview_refresh(&mut self) {
        let Some((generation, rx)) = &self.preview_job else {
            return;
        };
        match rx.try_recv() {
            Ok(outcome) if outcome.generation == *generation => {
                self.preview_job = None;
                self.apply_preview_outcome(outcome);
            }
            Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => self.preview_job = None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    fn preview_inputs(&mut self) -> PreviewInputs {
        self.preview_generation = self.preview_generation.wrapping_add(1);
        let now_utc = Utc::now();
        self.last_milestone_refresh_minute = Some(now_utc.timestamp() / 60);
        let local = self.local_time_at(now_utc);
        let midnight = local
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight is a valid time");
        let midnight_utc = chrono::DateTime::<Utc>::from_naive_utc_and_offset(
            midnight - chrono::Duration::seconds(i64::from(local.offset().local_minus_utc())),
            Utc,
        );
        PreviewInputs {
            generation: self.preview_generation,
            config: self.config.clone(),
            now_utc,
            midnight_utc,
            weather_multiplier: self
                .status
                .as_ref()
                .and_then(|s| s.weather.as_ref())
                .and_then(|w| w.multiplier),
        }
    }

    fn apply_preview_outcome(&mut self, outcome: PreviewOutcome) {
        let curve_changed = self.apply_policy_previews(outcome.previews);
        match outcome.milestones {
            Ok(milestones) => {
                self.monitor_milestone_error = None;
                self.monitor_milestones = milestones;
                self.monitor_milestones_dirty = false;
                if let Some(schedule) = self.selected_monitor_schedule() {
                    let max_index = schedule.milestones.len().saturating_sub(1);
                    if self.selected_monitor_milestone > max_index {
                        self.selected_monitor_milestone = max_index;
                    }
                } else {
                    self.selected_monitor_milestone = 0;
                }
            }
            Err(error) => {
                self.monitor_milestones.clear();
                self.monitor_milestone_error = Some(error);
                self.monitor_milestones_dirty = false;
                self.selected_monitor_milestone = 0;
            }
        }
        // Start the morph only once the new data is in place, so its whole
        // duration is visible.
        if curve_changed {
            self.motion.trigger(
                super::motion::TransientKind::CurveMorph,
                Instant::now(),
                Duration::from_millis(320),
            );
        }
    }

    /// Installs new target previews and curves. Returns whether the selected
    /// monitor's curve visibly changed, keeping the previous curve for a morph.
    fn apply_policy_previews(&mut self, previews: Result<PolicyPreviews, String>) -> bool {
        let Ok((targets, curves)) = previews else {
            self.monitor_targets_now.clear();
            self.monitor_curves.clear();
            return false;
        };

        let selected = self.selected_monitor_logical_id().map(str::to_owned);
        let previous = selected.as_ref().and_then(|id| {
            self.monitor_curves
                .iter()
                .find(|curve| &curve.logical_id == id)
                .cloned()
        });
        let mut changed_curve = false;
        if let (Some(previous), Some(id)) = (previous, selected) {
            let changed = curves
                .iter()
                .find(|curve| curve.logical_id == id)
                .is_some_and(|next| {
                    next.samples.len() == previous.samples.len()
                        && next
                            .samples
                            .iter()
                            .zip(&previous.samples)
                            .any(|(a, b)| (a - b).abs() > 0.5)
                });
            if changed {
                self.curve_morph_from = Some(previous);
                changed_curve = true;
            }
        }
        self.monitor_targets_now = targets;
        self.monitor_curves = curves;
        changed_curve
    }

    /// Determines whether a milestone's effective scheduled time has been constrained
    /// by monotonicity or day boundaries, differing from the requested solar+offset time.
    #[must_use]
    pub fn is_milestone_constrained(
        &self,
        logical_id: &str,
        milestone: &crate::policy::MonitorMilestone,
    ) -> bool {
        let requested_offset = self
            .config
            .monitors
            .iter()
            .find(|m| m.logical_id == logical_id)
            .and_then(|m| {
                m.milestone_adjustments
                    .iter()
                    .find(|a| a.milestone == milestone.milestone)
            })
            .map_or(0, |a| a.minutes_offset);

        milestone.is_constrained(requested_offset)
    }
}

fn load_timezone(timezone_name: &str) -> Option<tz::TimeZone> {
    let path = std::path::Path::new("/usr/share/zoneinfo").join(timezone_name);
    std::fs::read(path)
        .ok()
        .and_then(|data| tz::TimeZone::from_tz_data(&data).ok())
        .or_else(|| tz::TimeZone::from_posix_tz(timezone_name).ok())
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything a background recomputation needs, captured on the UI thread.
pub(crate) struct PreviewInputs {
    generation: u64,
    config: Config,
    now_utc: chrono::DateTime<Utc>,
    midnight_utc: chrono::DateTime<Utc>,
    weather_multiplier: Option<f64>,
}

pub(crate) struct PreviewOutcome {
    generation: u64,
    milestones: Result<Vec<MonitorMilestoneSchedule>, String>,
    previews: Result<PolicyPreviews, String>,
}

fn compute_preview_outcome(inputs: &PreviewInputs) -> PreviewOutcome {
    PreviewOutcome {
        generation: inputs.generation,
        previews: build_policy_previews(
            &inputs.config,
            inputs.now_utc,
            inputs.midnight_utc,
            inputs.weather_multiplier,
        ),
        milestones: build_monitor_milestones(
            &inputs.config,
            inputs.now_utc,
            inputs.weather_multiplier,
        ),
    }
}

type PolicyPreviews = (
    Vec<super::model::TargetPreview>,
    Vec<super::model::CurvePreview>,
);

fn build_policy_previews(
    config: &Config,
    now_utc: chrono::DateTime<Utc>,
    midnight_utc: chrono::DateTime<Utc>,
    weather_multiplier: Option<f64>,
) -> Result<PolicyPreviews, String> {
    use super::model::{CurvePreview, TargetPreview, CURVE_SAMPLE_MINUTES};

    if config.monitors.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    config.validate().map_err(|error| error.to_string())?;
    let location = Location::from_timezone_name(
        config.location.latitude,
        config.location.longitude,
        &config.location.timezone,
    )
    .map_err(|error| error.to_string())?;
    let evaluate = |at: chrono::DateTime<Utc>, multiplier: Option<f64>| {
        policy::compute_policy(&PolicyContext {
            now_utc: at,
            location: &location,
            config: &config.solar_policy,
            weather_multiplier: multiplier,
            monitors: &config.monitors,
        })
        .map_err(|error| error.to_string())
    };

    let solar = evaluate(now_utc, None)?;
    let weather = match weather_multiplier {
        Some(multiplier) => evaluate(now_utc, Some(multiplier))?,
        None => solar.clone(),
    };
    let targets = solar
        .targets
        .iter()
        .zip(&weather.targets)
        .map(|(solar, weather)| TargetPreview {
            logical_id: solar.logical_id.clone(),
            solar_percent: solar.percent,
            weather_percent: weather.percent,
        })
        .collect();

    let sample_count = (24 * 60 / CURVE_SAMPLE_MINUTES + 1) as usize;
    let mut curves: Vec<CurvePreview> = config
        .monitors
        .iter()
        .map(|monitor| CurvePreview {
            logical_id: monitor.logical_id.clone(),
            samples: Vec::with_capacity(sample_count),
        })
        .collect();
    for index in 0..sample_count {
        let at = midnight_utc
            + chrono::Duration::minutes(i64::from(CURVE_SAMPLE_MINUTES) * index as i64);
        let output = evaluate(at, None)?;
        for (curve, target) in curves.iter_mut().zip(&output.targets) {
            curve.samples.push(f32::from(target.percent));
        }
    }
    Ok((targets, curves))
}

fn build_monitor_milestones(
    config: &Config,
    now_utc: chrono::DateTime<Utc>,
    weather_multiplier: Option<f64>,
) -> Result<Vec<MonitorMilestoneSchedule>, String> {
    config.validate().map_err(|error| error.to_string())?;
    let location = Location::from_timezone_name(
        config.location.latitude,
        config.location.longitude,
        &config.location.timezone,
    )
    .map_err(|error| error.to_string())?;

    policy::compute_monitor_milestones(&PolicyContext {
        now_utc,
        location: &location,
        config: &config.solar_policy,
        weather_multiplier,
        monitors: &config.monitors,
    })
    .map_err(|error| error.to_string())
}
