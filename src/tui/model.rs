use std::sync::mpsc;
use std::time::Instant;

use crate::config::Config;
use crate::ipc::StatusResponse;
use crate::policy::MonitorMilestoneSchedule;

use super::form::FormState;
use super::worker::{IpcCommand, IpcEvent};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Clone, Default)]
pub enum ActiveModal {
    #[default]
    None,
    ThemeSelect(ratatui::widgets::ListState, crate::tui::theme::Theme),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DaemonConnection {
    Unknown,
    Connected,
    Disconnected,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActiveInputKind {
    Decimal,
    Integer,
    Time,
    Text,
    Secret,
    Toggle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Monitors,
    Limits,
    Location,
    Weather,
    Settings,
}

impl Tab {
    pub const ALL: [Self; 5] = [
        Self::Monitors,
        Self::Limits,
        Self::Location,
        Self::Weather,
        Self::Settings,
    ];

    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Monitors => "Monitors",
            Self::Limits => "Limits",
            Self::Location => "Location",
            Self::Weather => "Weather",
            Self::Settings => "Settings",
        }
    }

    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Monitors => 0,
            Self::Limits => 1,
            Self::Location => 2,
            Self::Weather => 3,
            Self::Settings => 4,
        }
    }

    #[must_use]
    pub fn next(self) -> Self {
        let next_index = (self.index() + 1) % Self::ALL.len();
        Self::ALL[next_index]
    }

    #[must_use]
    pub fn previous(self) -> Self {
        let previous_index = if self.index() == 0 {
            Self::ALL.len() - 1
        } else {
            self.index() - 1
        };
        Self::ALL[previous_index]
    }
}

pub struct Model {
    pub should_quit: bool,
    pub status: Option<StatusResponse>,
    pub selected_monitor: usize,
    pub(super) ipc_tx: mpsc::SyncSender<IpcCommand>,
    pub(super) ipc_rx: mpsc::Receiver<IpcEvent>,
    pub daemon_connection: DaemonConnection,
    pub config: Config,
    pub config_error: Option<String>,
    pub active_tab: Tab,

    pub show_help: bool,
    pub input_mode: InputMode,
    pub active_setting: usize,
    pub(crate) form: FormState,
    pub monitor_advanced_open: bool,
    pub selected_monitor_milestone: usize,
    pub monitor_milestones: Vec<MonitorMilestoneSchedule>,
    pub monitor_milestone_error: Option<String>,
    pub(super) last_milestone_refresh_minute: Option<i64>,
    pub config_dirty: bool,
    pub last_config_mutation: Option<Instant>,
    pub active_modal: ActiveModal,
}
