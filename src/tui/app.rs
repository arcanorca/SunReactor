use std::time::{Duration, Instant};

use crate::config::MotionLevel;

use super::globe::{rotation_duration, GlobeCenter};
use super::worker::IpcCommand;

mod environment;
#[cfg(test)]
pub(crate) use environment::ModelEnvironment;
mod input_editing;
pub use input_editing::EditBufferPolicy;
mod milestones;
mod persistence;
mod preview;
pub(crate) use preview::PreviewOutcome;

use super::model::{
    settings_index, ActionKind, ActionState, DaemonConnection, DaemonLifecycle, ErrorCategory,
    Model, MonitorDiscoveryState, MonitorWorkspaceState, OperationalMode, Tab,
};

impl Model {
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

    pub(crate) fn selected_config_monitor_index(&self) -> Option<usize> {
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

    pub(crate) fn local_time_at(
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
}
