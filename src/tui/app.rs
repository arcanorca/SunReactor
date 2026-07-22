use std::time::{Duration, Instant};

use chrono::Utc;

use crate::config::{self as app_config, Config};
use crate::ipc::Request;
use crate::policy::{self, MonitorMilestoneSchedule, PolicyContext};
use crate::solar::Location;

use super::form::FormState;
use super::worker::{spawn_ipc_worker, IpcCommand};

use super::model::{ActiveInputKind, DaemonConnection, InputMode, Model, Tab};

impl Model {
    #[must_use]
    pub fn new() -> Self {
        let (config, config_error) = match app_config::load() {
            Ok(report) => (report.config, None),
            Err(error) => (Config::default(), Some(error.to_string())),
        };
        let form = FormState::new(&config);
        let (ipc_tx, ipc_rx) = spawn_ipc_worker(Duration::from_secs(2));

        let mut app = Self {
            should_quit: false,
            status: None,
            selected_monitor: 0,
            ipc_tx,
            ipc_rx,
            daemon_connection: DaemonConnection::Unknown,
            config,
            config_error,
            active_tab: Tab::Monitors,

            show_help: false,
            input_mode: InputMode::Normal,
            active_setting: 0,
            form,
            monitor_advanced_open: false,
            selected_monitor_milestone: 0,
            monitor_milestones: Vec::new(),
            monitor_milestone_error: None,
            last_milestone_refresh_minute: None,
            config_dirty: false,
            last_config_mutation: None,
            active_modal: super::model::ActiveModal::None,
        };
        app.refresh_monitor_milestones();
        app
    }

    #[must_use]
    pub fn automation_field_count(&self) -> usize {
        self.form.automation_field_count()
    }

    fn tab_field_count(&self, tab: Tab) -> usize {
        match tab {
            Tab::Monitors => 0,
            Tab::Limits => self.automation_field_count(),
            Tab::Location => 3,
            Tab::Weather => 1,
            Tab::Settings => 5,
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.active_setting = 0;
    }

    pub fn previous_tab(&mut self) {
        self.active_tab = self.active_tab.previous();
        self.active_setting = 0;
    }

    pub fn move_selection_down(&mut self) {
        match self.active_tab {
            Tab::Monitors => {
                if let Some(status) = &self.status {
                    if !status.monitors.is_empty()
                        && self.selected_monitor < status.monitors.len() - 1
                    {
                        self.selected_monitor += 1;
                        self.selected_monitor_milestone = 0;
                    }
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
                if let Some(status) = &self.status {
                    if !status.monitors.is_empty() && self.selected_monitor > 0 {
                        self.selected_monitor -= 1;
                        self.selected_monitor_milestone = 0;
                    }
                }
            }
            _ if self.active_setting > 0 => {
                self.active_setting -= 1;
            }
            _ => {}
        }
    }

    pub fn toggle_active_setting(&mut self) {
        if self.active_tab == Tab::Settings {
            if self.active_setting == 0 {
                let mut state = ratatui::widgets::ListState::default();
                let current_index = crate::tui::theme::Theme::ALL
                    .iter()
                    .position(|t| *t == self.config.tui.theme)
                    .unwrap_or(0);
                state.select(Some(current_index));
                self.active_modal =
                    super::model::ActiveModal::ThemeSelect(state, self.config.tui.theme);
            } else if self.active_setting == 1 {
                self.config.tui.use_12h_time = !self.config.tui.use_12h_time;
            } else if self.active_setting == 2 {
                self.config.tui.temperature_unit = match self.config.tui.temperature_unit {
                    crate::config::TemperatureUnit::Celsius => {
                        crate::config::TemperatureUnit::Fahrenheit
                    }
                    crate::config::TemperatureUnit::Fahrenheit => {
                        crate::config::TemperatureUnit::Celsius
                    }
                };
            } else if self.active_setting == 3 {
                self.config.daemon.smooth_transition = !self.config.daemon.smooth_transition;
            }
        }
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

    pub fn start_editing(&mut self) {
        if matches!(self.active_tab, Tab::Monitors) {
            return; // Monitors tab has no editable input fields
        }
        self.input_mode = InputMode::Editing;
    }

    pub fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn append_char_to_active_input(&mut self, c: char) {
        if let Some(input) = self.active_input_mut() {
            let mut value = input.value().to_string();
            value.push(c);
            *input = tui_input::Input::default().with_value(value);
        }
    }

    pub fn backspace_active_input(&mut self) {
        if let Some(input) = self.active_input_mut() {
            let mut value = input.value().to_string();
            if value.is_empty() {
                return;
            }

            value.pop();
            *input = tui_input::Input::default().with_value(value);
        }
    }

    #[must_use]
    pub fn active_input_kind(&self) -> Option<ActiveInputKind> {
        if matches!(self.active_tab, Tab::Monitors) {
            return None;
        }
        self.form
            .active_input_kind(self.active_tab, self.active_setting)
    }

    #[must_use]
    pub fn active_input_ref(&self) -> Option<&tui_input::Input> {
        if matches!(self.active_tab, Tab::Monitors) {
            return None;
        }
        self.form
            .active_input_ref(self.active_tab, self.active_setting)
    }

    pub fn active_input_mut(&mut self) -> Option<&mut tui_input::Input> {
        if matches!(self.active_tab, Tab::Monitors) {
            return None;
        }
        self.form
            .active_input_mut(self.active_tab, self.active_setting)
    }

    pub fn send_command(&self, request: Request) {
        let _ = self.ipc_tx.try_send(IpcCommand::Send(request));
    }

    pub fn check_debounced_save(&mut self) {
        if self.config_dirty {
            if let Some(mutation_time) = self.last_config_mutation {
                if mutation_time.elapsed() >= Duration::from_millis(400) {
                    self.config_dirty = false;
                    self.last_config_mutation = None;
                    self.save_config();
                }
            }
        }
    }

    pub fn save_config(&mut self) {
        self.form.apply_to_config(&mut self.config);

        match app_config::save(&self.config) {
            Ok(_) => {
                self.config_error = None;
                self.config_dirty = false;
                self.last_config_mutation = None;

                self.form.refresh_from_config(&self.config);
                self.refresh_monitor_milestones();
                self.send_command(Request::ReloadConfig);
            }
            Err(error) => {
                self.config_error = Some(error.to_string());
                self.config_dirty = false;
                self.last_config_mutation = None;
            }
        }
    }

    pub fn increase_monitor_gamma(&mut self) {
        if let Some(logical_id) = self.selected_monitor_logical_id().map(String::from) {
            if let Some(monitor) = self
                .config
                .monitors
                .iter_mut()
                .find(|m| m.logical_id == logical_id)
            {
                monitor.transition_gamma = (monitor.transition_gamma + 0.1).clamp(0.1, 3.0);
                monitor.transition_gamma = (monitor.transition_gamma * 10.0).round() / 10.0;
                self.config_dirty = true;
                self.last_config_mutation = Some(Instant::now());
                self.refresh_monitor_milestones();
            }
        }
    }

    pub fn decrease_monitor_gamma(&mut self) {
        if let Some(logical_id) = self.selected_monitor_logical_id().map(String::from) {
            if let Some(monitor) = self
                .config
                .monitors
                .iter_mut()
                .find(|m| m.logical_id == logical_id)
            {
                monitor.transition_gamma = (monitor.transition_gamma - 0.1).clamp(0.1, 3.0);
                monitor.transition_gamma = (monitor.transition_gamma * 10.0).round() / 10.0;
                self.config_dirty = true;
                self.last_config_mutation = Some(Instant::now());
                self.refresh_monitor_milestones();
            }
        }
    }

    pub fn suspend_writes(&mut self) {
        if let Ok(minutes) = self.form.suspend_duration_minutes() {
            self.send_command(Request::Suspend { minutes });
        }
    }

    pub fn resume_writes(&mut self) {
        self.send_command(Request::Resume);
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
        self.status
            .as_ref()
            .and_then(|status| status.monitors.get(self.selected_monitor))
            .map(|monitor| monitor.logical_id.as_str())
            .or_else(|| {
                self.config
                    .monitors
                    .get(self.selected_monitor)
                    .map(|monitor| monitor.logical_id.as_str())
            })
    }

    pub fn refresh_monitor_milestones_if_needed(&mut self) {
        let minute = Utc::now().timestamp() / 60;
        if self.last_milestone_refresh_minute == Some(minute) {
            return;
        }
        self.refresh_monitor_milestones();
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

    pub fn refresh_monitor_milestones(&mut self) {
        self.last_milestone_refresh_minute = Some(Utc::now().timestamp() / 60);

        let weather_multiplier = self
            .status
            .as_ref()
            .and_then(|s| s.weather.as_ref())
            .and_then(|w| w.multiplier);

        let preview = build_monitor_milestones(&self.config, Utc::now(), weather_multiplier);
        match preview {
            Ok(milestones) => {
                self.monitor_milestone_error = None;
                self.monitor_milestones = milestones;
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
                self.selected_monitor_milestone = 0;
            }
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
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
