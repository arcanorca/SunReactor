use std::sync::mpsc;
use std::time::Instant;

use crate::config::Config;
use crate::discovery::DiscoveryReport;
use crate::ipc::StatusResponse;
use crate::policy::MonitorMilestoneSchedule;

use super::form::FormState;
use super::worker::{IpcCommand, IpcEvent};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MonitorPaneFocus {
    #[default]
    List,
    Detail,
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

/// Discovery is an explicit UI state because hardware probing is slow and
/// must not be inferred from an empty daemon status snapshot.
#[derive(Debug, Clone, Default)]
pub(crate) enum MonitorDiscoveryState {
    #[default]
    NotRequested,
    Loading,
    Complete(Box<DiscoveryReport>),
}

/// The monitor workspace needs to distinguish configuration and hardware
/// evidence instead of turning every empty list into "no monitors detected".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonitorWorkspaceState {
    Connecting,
    DaemonUnavailable,
    Discovering,
    DiscoveredButUnconfigured {
        compatible_count: usize,
        importable_count: usize,
        incomplete_ddc_probe: bool,
    },
    NoCompatibleHardware,
    ConfiguredUnavailable,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonLifecycle {
    Active,
    IdleDimmed,
    Suspended,
    Unreachable,
}

impl DaemonLifecycle {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::IdleDimmed => "idle dimmed",
            Self::Suspended => "suspended",
            Self::Unreachable => "unreachable",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "LIVE",
            Self::IdleDimmed => "IDLE",
            Self::Suspended => "SUSPENDED",
            Self::Unreachable => "OFFLINE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalMode {
    Offline,
    Suspended,
    IdleDimmed,
    Override,
    Automatic,
}

impl OperationalMode {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Offline => "OFFLINE",
            Self::Suspended => "SUSPENDED",
            Self::IdleDimmed => "IDLE DIMMED",
            Self::Override => "OVERRIDE",
            Self::Automatic => "AUTOMATIC",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextMilestoneInfo {
    pub milestone_label: &'static str,
    pub target_percent: u8,
    pub time_str: String,
    pub minutes_offset: i16,
    pub is_tomorrow: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Monitors,
    Limits,
    Location,
    Weather,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Suspend,
    Resume,
    SaveConfig,
    ReloadConfig,
    RefreshWeather,
}

impl ActionKind {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Suspend => "Suspend",
            Self::Resume => "Resume",
            Self::SaveConfig => "Save config",
            Self::ReloadConfig => "Reload config",
            Self::RefreshWeather => "Refresh weather",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Validation,
    ConfigWrite,
    DaemonUnavailable,
    DaemonRejected,
    Timeout,
    Transport,
}

impl ErrorCategory {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Validation => "Validation error",
            Self::ConfigWrite => "Save error",
            Self::DaemonUnavailable => "Daemon unavailable",
            Self::DaemonRejected => "Daemon rejected",
            Self::Timeout => "Timeout",
            Self::Transport => "IPC error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActionState {
    #[default]
    Idle,
    Pending {
        command_id: u64,
        action: ActionKind,
        description: String,
        started_at: Instant,
    },
    Success {
        action: ActionKind,
        message: String,
        completed_at: Instant,
    },
    Warning {
        action: ActionKind,
        category: ErrorCategory,
        message: String,
        completed_at: Instant,
    },
    Error {
        action: ActionKind,
        category: ErrorCategory,
        message: String,
        completed_at: Instant,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponsiveMode {
    Comfortable,
    Compact,
    Minimal,
    TooSmall,
}

impl ResponsiveMode {
    #[must_use]
    pub fn from_size(width: u16, height: u16) -> Self {
        if width < 40 || height < 12 {
            Self::TooSmall
        } else if width >= 80 && height >= 24 {
            Self::Comfortable
        } else if width >= 60 && height >= 18 {
            Self::Compact
        } else {
            Self::Minimal
        }
    }
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
            Self::Limits => "Automation",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutomationRegionFocus {
    /// The horizontal monitor chips; ←/→ switch monitor.
    #[default]
    Selector,
    /// The solar curvature (`transition_gamma`) control.
    Curve,
    Milestones,
}

/// The automation target each monitor would receive now, with and without
/// the current weather input. Computed by the shared policy engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPreview {
    pub logical_id: String,
    pub solar_percent: u8,
    pub weather_percent: u8,
}

/// Today's solar automation curve for one monitor, sampled from local
/// midnight every [`CURVE_SAMPLE_MINUTES`] minutes (weather excluded).
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePreview {
    pub logical_id: String,
    pub samples: Vec<f32>,
}

pub const CURVE_SAMPLE_MINUTES: u32 = 15;

/// Stable indexes for the shared Settings form. The form API uses indexes, but
/// callers should not need to remember their presentation order.
pub(crate) mod settings_index {
    pub const THEME: usize = 0;
    pub const EFFECTS: usize = 1;
    pub const SHOW_LOGO: usize = 2;
    pub const REFRESH_RATE: usize = 3;
    pub const TIME_FORMAT: usize = 4;
    pub const TEMPERATURE_UNIT: usize = 5;
    pub const IDLE_DIM: usize = 6;
    pub const SUSPEND_DURATION: usize = 7;
    pub const WEATHER_ENABLED: usize = 8;
    pub const WEATHER_API_KEY: usize = 9;
    pub const COUNT: usize = 10;
}

pub struct Model {
    pub should_quit: bool,
    pub status: Option<StatusResponse>,
    pub selected_monitor: usize,
    /// Stable configured-monitor identity for reconciling status snapshots that
    /// arrive in a different order than the previous snapshot.
    pub(crate) selected_monitor_id: Option<String>,
    pub(super) ipc_tx: mpsc::SyncSender<IpcCommand>,
    pub(super) ipc_rx: mpsc::Receiver<IpcEvent>,
    pub daemon_connection: DaemonConnection,
    pub(crate) monitor_discovery: MonitorDiscoveryState,
    pub config: Config,
    pub config_error: Option<String>,
    pub active_tab: Tab,

    pub show_help: bool,
    pub help_scroll: usize,
    pub monitor_list_state: ratatui::widgets::ListState,
    pub milestone_list_state: ratatui::widgets::ListState,
    pub settings_scroll: usize,
    pub input_mode: InputMode,
    pub active_setting: usize,
    pub action_state: ActionState,
    pub next_command_id: u64,
    pub(crate) form: FormState,
    pub(crate) editing_form_snapshot: Option<FormState>,
    pub monitor_pane_focus: MonitorPaneFocus,
    pub monitor_control_index: usize,
    pub automation_focus: AutomationRegionFocus,
    pub selected_monitor_milestone: usize,
    pub monitor_milestones: Vec<MonitorMilestoneSchedule>,
    pub monitor_milestone_error: Option<String>,
    pub(super) last_milestone_refresh_minute: Option<i64>,
    /// The displayed milestone table no longer represents the in-memory monitor policy.
    /// It is rebuilt after the active keyboard adjustment settles, not for every key repeat.
    pub(crate) monitor_milestones_dirty: bool,
    pub config_dirty: bool,
    pub last_config_mutation: Option<Instant>,
    pub active_modal: ActiveModal,
    pub motion: super::motion::UiMotionState,
    pub last_weather_fetched_at: Option<u64>,
    /// Parsed once when the location configuration is loaded or committed.
    /// Rendering only queries this cache for the current offset.
    pub(super) timezone_cache: Option<tz::TimeZone>,
    pub cell_metrics: super::geometry::TerminalCellMetrics,
    /// `None` saves to the user's configuration file.
    pub(crate) config_save_path: Option<std::path::PathBuf>,
    pub monitor_targets_now: Vec<TargetPreview>,
    pub monitor_curves: Vec<CurvePreview>,
    /// The curve shown before the latest recompute, kept for a bounded morph.
    pub(crate) curve_morph_from: Option<CurvePreview>,
    /// Background schedule/curve recomputation. Solar schedule resolution is
    /// too slow for the input thread, so its result is applied on a later tick.
    pub(crate) preview_job: Option<(u64, mpsc::Receiver<super::app::PreviewOutcome>)>,
    pub(crate) preview_generation: u64,
    /// Keyboard focus is on the tab bar, where ←/→ switch workspaces.
    pub(crate) tabs_focused: bool,
    /// Monitors draws its selector as a horizontal switcher (narrow layout),
    /// so ←/→ rather than ↑/↓ change monitor there.
    pub(crate) monitor_selector_horizontal: bool,
}

#[cfg(test)]
mod tests {
    use super::ResponsiveMode;

    #[test]
    fn test_responsive_mode_detection() {
        assert_eq!(
            ResponsiveMode::from_size(80, 24),
            ResponsiveMode::Comfortable
        );
        assert_eq!(
            ResponsiveMode::from_size(100, 30),
            ResponsiveMode::Comfortable
        );
        assert_eq!(ResponsiveMode::from_size(60, 20), ResponsiveMode::Compact);
        assert_eq!(ResponsiveMode::from_size(70, 18), ResponsiveMode::Compact);
        assert_eq!(ResponsiveMode::from_size(40, 16), ResponsiveMode::Minimal);
        assert_eq!(ResponsiveMode::from_size(50, 14), ResponsiveMode::Minimal);
        assert_eq!(ResponsiveMode::from_size(35, 16), ResponsiveMode::TooSmall);
        assert_eq!(ResponsiveMode::from_size(60, 10), ResponsiveMode::TooSmall);
        assert_eq!(ResponsiveMode::from_size(20, 8), ResponsiveMode::TooSmall);
    }
}
