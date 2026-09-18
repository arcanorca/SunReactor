use crate::tui::model::{InputMode, Model, MonitorPaneFocus, Tab};

/// Semantic identifier for user interface actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    SwitchWorkspace,
    CycleWorkspace,
    FocusWorkspace,
    ToggleHelp,
    Quit,
    SelectMonitor,
    FocusDetail,
    BackToList,
    OpenAutomation,
    SuspendResumeWrites,
    AdjustGamma,
    AdjustBrightnessRange,
    CycleMonitor,
    SelectControlField,
    SelectSetting,
    ToggleOrEditSetting,
    SelectMilestone,
    AdjustMilestoneOffset,
    ResetMilestoneOffset,
    ReturnToMonitors,
    SelectLocationField,
    EditField,
    SearchCities,
    SelectCityMatch,
    AdoptCity,
    MoveCursor,
    InputDigits,
    InputText,
    DeleteChar,
    SaveField,
    CancelEdit,
    ScrollHelp,
    ScrollHelpPage,
    ScrollHelpJump,
    CloseHelp,
    RetryWeather,
}

/// Structural operational context of the user interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiCommandContext {
    /// Any tab: keyboard focus on the tab bar
    TabBar,
    /// Tab 2: Automation - monitor chips
    AutomationSelector,
    /// Tab 1: Monitors - Master list navigation
    MonitorsList,
    /// Tab 1: Monitors - Operating range / controls detail pane
    MonitorsDetail,
    /// Tab 1: Monitors - Editing Operating Range (min/max percentage)
    MonitorsEdit,
    /// Tab 2: Automation (Limits) - Milestone schedule and time offsets
    Automation,
    /// Tab 2: Automation (Limits) - Solar-to-brightness curve control
    AutomationCurve,
    /// Tab 2: Automation (Limits) - In-place editing mode
    AutomationEdit,
    /// Tab 3: Location - Coordinates & timezone field selection
    LocationNav,
    /// Tab 3: Location - City autocomplete search input active
    LocationCityEdit,
    /// Tab 3: Location - Lat / Lon / Timezone numeric or text edit
    LocationFieldEdit,
    /// Tab 4: Weather - Read-only weather input and its brightness effect
    WeatherObservational,
    /// Tab 5: Settings - Preference navigation and toggles
    SettingsNav,
    /// Tab 5: Settings - Input editing (FPS, timeouts, API key)
    SettingsEdit,
    /// Any Tab: Help modal overlay open
    HelpModal,
}

impl UiCommandContext {
    /// Authoritatively derives the current operational workspace context.
    #[must_use]
    pub fn for_workspace(app: &Model) -> Self {
        if app.input_mode == InputMode::Normal && app.tabs_focused {
            return Self::TabBar;
        }
        match app.input_mode {
            InputMode::Editing => match app.active_tab {
                Tab::Monitors => Self::MonitorsEdit,
                Tab::Location => {
                    if app.active_setting == 0 {
                        Self::LocationCityEdit
                    } else {
                        Self::LocationFieldEdit
                    }
                }
                Tab::Settings => Self::SettingsEdit,
                Tab::Limits => Self::AutomationEdit,
                Tab::Weather => Self::WeatherObservational,
            },
            InputMode::Normal => match app.active_tab {
                Tab::Monitors => {
                    if matches!(app.monitor_pane_focus, MonitorPaneFocus::Detail) {
                        Self::MonitorsDetail
                    } else {
                        Self::MonitorsList
                    }
                }
                Tab::Limits => match app.automation_focus {
                    crate::tui::model::AutomationRegionFocus::Selector => Self::AutomationSelector,
                    crate::tui::model::AutomationRegionFocus::Curve => Self::AutomationCurve,
                    crate::tui::model::AutomationRegionFocus::Milestones => Self::Automation,
                },
                Tab::Location => Self::LocationNav,
                Tab::Weather => Self::WeatherObservational,
                Tab::Settings => Self::SettingsNav,
            },
        }
    }

    /// Authoritatively derives the footer interaction context.
    #[must_use]
    pub fn for_footer(app: &Model) -> Self {
        if app.show_help {
            Self::HelpModal
        } else {
            Self::for_workspace(app)
        }
    }

    /// User-facing section title for the active context.
    #[must_use]
    pub const fn title(&self) -> &'static str {
        match self {
            Self::TabBar => "Workspaces",
            Self::AutomationSelector => "Automation monitor",
            Self::MonitorsList => "Monitors",
            Self::MonitorsDetail => "Brightness range",
            Self::MonitorsEdit => "Edit brightness range",
            Self::Automation => "Automation schedule",
            Self::AutomationCurve => "Solar curvature",
            Self::AutomationEdit => "Edit gamma",
            Self::LocationNav => "Location",
            Self::LocationCityEdit => "Search city",
            Self::LocationFieldEdit => "Edit location",
            Self::WeatherObservational => "Weather",
            Self::SettingsNav => "Settings",
            Self::SettingsEdit => "Edit setting",
            Self::HelpModal => "Help",
        }
    }
}

/// Specification for an operator command in the shared command presentation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// Semantic identifier for the command
    pub id: CommandId,
    /// Full key display representation for Help overlay
    pub keys: &'static str,
    /// Compact key representation for narrow footers
    pub compact_keys: &'static str,
    /// Detailed action description for the Help reference
    pub action: &'static str,
    /// Concise action label for the footer command bar
    pub footer_label: &'static str,
}

/// Grouped commands for a given UI context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextCommands {
    pub context: UiCommandContext,
    pub current_commands: &'static [CommandSpec],
    pub global_commands: &'static [CommandSpec],
}

// ── Shared Command Definitions (Presentation Source) ───────────────────────

const GLOBAL_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::CycleWorkspace,
        keys: "Tab / Shift+Tab",
        compact_keys: "Tab",
        action: "Next / previous workspace (also Ctrl+Tab, Ctrl+PgDn / Ctrl+PgUp)",
        footer_label: "Workspace",
    },
    CommandSpec {
        id: CommandId::SwitchWorkspace,
        keys: "1–5",
        compact_keys: "1-5",
        action: "Jump to a workspace",
        footer_label: "Jump",
    },
    CommandSpec {
        id: CommandId::ToggleHelp,
        keys: "?",
        compact_keys: "?",
        action: "Toggle help reference",
        footer_label: "Help",
    },
    CommandSpec {
        id: CommandId::Quit,
        keys: "q",
        compact_keys: "q",
        action: "Quit SunReactor",
        footer_label: "Quit",
    },
];

const TAB_BAR_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::CycleWorkspace,
        keys: "← →",
        compact_keys: "←→",
        action: "Move between workspaces",
        footer_label: "Workspace",
    },
    CommandSpec {
        id: CommandId::FocusWorkspace,
        keys: "↓ / Enter",
        compact_keys: "↓",
        action: "Go into the workspace",
        footer_label: "Open",
    },
];

const MONITORS_SWITCHER_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::SelectMonitor,
        keys: "← →",
        compact_keys: "←→",
        action: "Select monitor",
        footer_label: "Select",
    },
    CommandSpec {
        id: CommandId::FocusDetail,
        keys: "↓ / Enter",
        compact_keys: "↓",
        action: "Edit brightness range",
        footer_label: "Range",
    },
    CommandSpec {
        id: CommandId::OpenAutomation,
        keys: "a",
        compact_keys: "a",
        action: "Open automation for this monitor",
        footer_label: "Automation",
    },
    CommandSpec {
        id: CommandId::SuspendResumeWrites,
        keys: "s / r",
        compact_keys: "s/r",
        action: "Suspend / Resume writes",
        footer_label: "Susp/Res",
    },
];

const AUTOMATION_SELECTOR_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::CycleMonitor,
        keys: "← →",
        compact_keys: "←→",
        action: "Switch monitor",
        footer_label: "Monitor",
    },
    CommandSpec {
        id: CommandId::FocusDetail,
        keys: "↓ / Enter",
        compact_keys: "↓",
        action: "Go to this monitor's solar curvature",
        footer_label: "Curvature",
    },
];

const MONITORS_LIST_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::SelectMonitor,
        keys: "↑↓ / jk",
        compact_keys: "↑↓",
        action: "Select monitor",
        footer_label: "Select",
    },
    CommandSpec {
        id: CommandId::FocusDetail,
        keys: "→ / Enter",
        compact_keys: "→",
        action: "Edit brightness range",
        footer_label: "Range",
    },
    CommandSpec {
        id: CommandId::OpenAutomation,
        keys: "a",
        compact_keys: "a",
        action: "Open automation for this monitor",
        footer_label: "Automation",
    },
    CommandSpec {
        id: CommandId::SuspendResumeWrites,
        keys: "s / r",
        compact_keys: "s/r",
        action: "Suspend / Resume writes",
        footer_label: "Susp/Res",
    },
];

const MONITORS_DETAIL_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::SelectControlField,
        keys: "↑↓ / jk",
        compact_keys: "↑↓",
        action: "Choose minimum or maximum",
        footer_label: "Field",
    },
    CommandSpec {
        id: CommandId::AdjustBrightnessRange,
        keys: "← →",
        compact_keys: "←→",
        action: "Adjust selected brightness ±1%",
        footer_label: "±1%",
    },
    CommandSpec {
        id: CommandId::EditField,
        keys: "Enter",
        compact_keys: "↵",
        action: "Enter exact brightness value",
        footer_label: "Edit",
    },
    CommandSpec {
        id: CommandId::BackToList,
        keys: "Esc",
        compact_keys: "Esc",
        action: "Back to monitor list",
        footer_label: "Back",
    },
];

const MONITORS_EDIT_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::MoveCursor,
        keys: "← →",
        compact_keys: "←→",
        action: "Move cursor",
        footer_label: "Move",
    },
    CommandSpec {
        id: CommandId::InputDigits,
        keys: "0–9",
        compact_keys: "0-9",
        action: "Input percentage value",
        footer_label: "Digits",
    },
    CommandSpec {
        id: CommandId::DeleteChar,
        keys: "Backspace",
        compact_keys: "Bksp",
        action: "Delete character",
        footer_label: "Del",
    },
    CommandSpec {
        id: CommandId::SaveField,
        keys: "Enter",
        compact_keys: "↵",
        action: "Save and apply",
        footer_label: "Save",
    },
    CommandSpec {
        id: CommandId::CancelEdit,
        keys: "Esc",
        compact_keys: "Esc",
        action: "Cancel editing",
        footer_label: "Cancel",
    },
];

const AUTOMATION_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::CycleMonitor,
        keys: "[ / ]",
        compact_keys: "[/]",
        action: "Switch monitor",
        footer_label: "Monitor",
    },
    CommandSpec {
        id: CommandId::SelectMilestone,
        keys: "↑↓ / jk",
        compact_keys: "↑↓",
        action: "Select milestone (↑ from the first returns to the curve)",
        footer_label: "Select",
    },
    CommandSpec {
        id: CommandId::AdjustMilestoneOffset,
        keys: "← →",
        compact_keys: "←→",
        action: "Shift the selected milestone by 1 minute",
        footer_label: "±1 min",
    },
    CommandSpec {
        id: CommandId::ResetMilestoneOffset,
        keys: "r",
        compact_keys: "r",
        action: "Reset the milestone to its solar time",
        footer_label: "Reset",
    },
];

const AUTOMATION_CURVE_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::CycleMonitor,
        keys: "[ / ]",
        compact_keys: "[/]",
        action: "Switch monitor",
        footer_label: "Monitor",
    },
    CommandSpec {
        id: CommandId::AdjustGamma,
        keys: "← →",
        compact_keys: "←→",
        action: "Adjust gamma by 0.05",
        footer_label: "Gamma",
    },
    CommandSpec {
        id: CommandId::EditField,
        keys: "Enter",
        compact_keys: "↵",
        action: "Enter exact gamma",
        footer_label: "Edit",
    },
    CommandSpec {
        id: CommandId::SelectMilestone,
        keys: "↓ / j",
        compact_keys: "↓",
        action: "Move to the schedule",
        footer_label: "Schedule",
    },
];

const AUTOMATION_EDIT_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::MoveCursor,
        keys: "← →",
        compact_keys: "←→",
        action: "Move cursor",
        footer_label: "Move",
    },
    CommandSpec {
        id: CommandId::SaveField,
        keys: "Enter",
        compact_keys: "↵",
        action: "Commit adjustment",
        footer_label: "Save",
    },
    CommandSpec {
        id: CommandId::CancelEdit,
        keys: "Esc",
        compact_keys: "Esc",
        action: "Cancel editing",
        footer_label: "Cancel",
    },
];

const LOCATION_NAV_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::SelectLocationField,
        keys: "↑↓ / jk",
        compact_keys: "↑↓",
        action: "Select location field",
        footer_label: "Select",
    },
    CommandSpec {
        id: CommandId::EditField,
        keys: "Enter",
        compact_keys: "↵",
        action: "Edit selected field",
        footer_label: "Edit",
    },
];

const LOCATION_CITY_EDIT_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::SearchCities,
        keys: "Type",
        compact_keys: "Type",
        action: "Search worldwide cities",
        footer_label: "Search",
    },
    CommandSpec {
        id: CommandId::SelectCityMatch,
        keys: "↑↓",
        compact_keys: "↑↓",
        action: "Select autocomplete match",
        footer_label: "Match",
    },
    CommandSpec {
        id: CommandId::AdoptCity,
        keys: "Enter",
        compact_keys: "↵",
        action: "Use city, coordinates, and timezone",
        footer_label: "Use",
    },
    CommandSpec {
        id: CommandId::CancelEdit,
        keys: "Esc",
        compact_keys: "Esc",
        action: "Cancel editing",
        footer_label: "Cancel",
    },
];

const LOCATION_FIELD_EDIT_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::InputText,
        keys: "Type",
        compact_keys: "Type",
        action: "Input coordinate or timezone",
        footer_label: "Type",
    },
    CommandSpec {
        id: CommandId::MoveCursor,
        keys: "← →",
        compact_keys: "←→",
        action: "Move cursor",
        footer_label: "Move",
    },
    CommandSpec {
        id: CommandId::SaveField,
        keys: "Enter",
        compact_keys: "↵",
        action: "Save and reload",
        footer_label: "Save",
    },
    CommandSpec {
        id: CommandId::CancelEdit,
        keys: "Esc",
        compact_keys: "Esc",
        action: "Cancel editing",
        footer_label: "Cancel",
    },
];

const WEATHER_COMMANDS: &[CommandSpec] = &[CommandSpec {
    id: CommandId::RetryWeather,
    keys: "r",
    compact_keys: "r",
    action: "Refresh weather",
    footer_label: "Refresh",
}];

const SETTINGS_NAV_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::SelectSetting,
        keys: "↑↓ / jk",
        compact_keys: "↑↓",
        action: "Select setting",
        footer_label: "Select",
    },
    CommandSpec {
        id: CommandId::ToggleOrEditSetting,
        keys: "Enter / Space",
        compact_keys: "↵",
        action: "Toggle or edit setting",
        footer_label: "Edit",
    },
    CommandSpec {
        id: CommandId::SuspendResumeWrites,
        keys: "s / r",
        compact_keys: "s/r",
        action: "Suspend / Resume writes",
        footer_label: "Susp/Res",
    },
];

const SETTINGS_EDIT_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::InputText,
        keys: "Type",
        compact_keys: "Type",
        action: "Input replacement value",
        footer_label: "Type",
    },
    CommandSpec {
        id: CommandId::SaveField,
        keys: "Enter",
        compact_keys: "↵",
        action: "Save configuration",
        footer_label: "Save",
    },
    CommandSpec {
        id: CommandId::CancelEdit,
        keys: "Esc",
        compact_keys: "Esc",
        action: "Cancel editing",
        footer_label: "Cancel",
    },
];

const HELP_MODAL_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::ScrollHelp,
        keys: "↑↓ / jk",
        compact_keys: "↑↓",
        action: "Scroll reference",
        footer_label: "Scroll",
    },
    CommandSpec {
        id: CommandId::ScrollHelpPage,
        keys: "PgUp / PgDn",
        compact_keys: "PgDn",
        action: "Scroll reference page",
        footer_label: "PgScroll",
    },
    CommandSpec {
        id: CommandId::ScrollHelpJump,
        keys: "Home / End",
        compact_keys: "Home",
        action: "Jump top / bottom",
        footer_label: "Jump",
    },
    CommandSpec {
        id: CommandId::CloseHelp,
        keys: "? / Esc / q",
        compact_keys: "Esc",
        action: "Close reference overlay",
        footer_label: "Close",
    },
];

fn commands_for(context: UiCommandContext) -> ContextCommands {
    match context {
        UiCommandContext::TabBar => ContextCommands {
            context,
            current_commands: TAB_BAR_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::AutomationSelector => ContextCommands {
            context,
            current_commands: AUTOMATION_SELECTOR_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::MonitorsList => ContextCommands {
            context,
            current_commands: MONITORS_LIST_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::MonitorsDetail => ContextCommands {
            context,
            current_commands: MONITORS_DETAIL_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::MonitorsEdit => ContextCommands {
            context,
            current_commands: MONITORS_EDIT_COMMANDS,
            global_commands: &[], // Keystrokes absorbed by buffer
        },
        UiCommandContext::Automation => ContextCommands {
            context,
            current_commands: AUTOMATION_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::AutomationCurve => ContextCommands {
            context,
            current_commands: AUTOMATION_CURVE_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::AutomationEdit => ContextCommands {
            context,
            current_commands: AUTOMATION_EDIT_COMMANDS,
            global_commands: &[],
        },
        UiCommandContext::LocationNav => ContextCommands {
            context,
            current_commands: LOCATION_NAV_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::LocationCityEdit => ContextCommands {
            context,
            current_commands: LOCATION_CITY_EDIT_COMMANDS,
            global_commands: &[],
        },
        UiCommandContext::LocationFieldEdit => ContextCommands {
            context,
            current_commands: LOCATION_FIELD_EDIT_COMMANDS,
            global_commands: &[],
        },
        UiCommandContext::WeatherObservational => ContextCommands {
            context,
            current_commands: WEATHER_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::SettingsNav => ContextCommands {
            context,
            current_commands: SETTINGS_NAV_COMMANDS,
            global_commands: GLOBAL_COMMANDS,
        },
        UiCommandContext::SettingsEdit => ContextCommands {
            context,
            current_commands: SETTINGS_EDIT_COMMANDS,
            global_commands: &[],
        },
        UiCommandContext::HelpModal => ContextCommands {
            context,
            current_commands: HELP_MODAL_COMMANDS,
            global_commands: &[],
        },
    }
}

/// Returns the presentation command specifications for the operational workspace.
///
/// Note: This is a shared presentation metadata source for Help and Footer,
/// not an execution or dispatch engine.
#[must_use]
pub fn commands_for_workspace(app: &Model) -> ContextCommands {
    let context = UiCommandContext::for_workspace(app);
    commands_for_app(app, context)
}

/// Returns the presentation command specifications for the footer bar.
#[must_use]
pub fn commands_for_footer(app: &Model) -> ContextCommands {
    let context = UiCommandContext::for_footer(app);
    commands_for_app(app, context)
}

fn commands_for_app(app: &Model, context: UiCommandContext) -> ContextCommands {
    let mut commands = commands_for(context);
    if matches!(context, UiCommandContext::MonitorsList) && app.monitor_selector_horizontal {
        commands.current_commands = MONITORS_SWITCHER_COMMANDS;
    }
    if matches!(context, UiCommandContext::WeatherObservational)
        && crate::tui::ui::weather_model::weather_action_label(app).is_none()
    {
        commands.current_commands = &[];
    }
    commands
}

/// Symbols shared by every SunReactor workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolLegendItem {
    pub symbol: &'static str,
    pub meaning: &'static str,
}

pub const OFFICIAL_SYMBOLS: &[SymbolLegendItem] = &[
    SymbolLegendItem {
        symbol: "❯",
        meaning: "keyboard cursor",
    },
    SymbolLegendItem {
        symbol: "›",
        meaning: "selected item in an unfocused list",
    },
    SymbolLegendItem {
        symbol: "‹ ›",
        meaning: "value changes with ← →",
    },
    SymbolLegendItem {
        symbol: "●",
        meaning: "current milestone · live · available",
    },
    SymbolLegendItem {
        symbol: "○",
        meaning: "inactive or unavailable",
    },
    SymbolLegendItem {
        symbol: "▲",
        meaning: "needs attention · suspended · stale",
    },
    SymbolLegendItem {
        symbol: "⎿",
        meaning: "detail for the row above",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{InputMode, Model, Tab};

    #[test]
    fn test_official_symbols_not_empty() {
        assert!(!OFFICIAL_SYMBOLS.is_empty());
        for item in OFFICIAL_SYMBOLS {
            assert!(!item.symbol.is_empty());
            assert!(!item.meaning.is_empty());
        }
    }

    #[test]
    fn test_single_source_of_truth_footer_and_help() {
        let mut model = Model::new();

        // Verify for every tab that commands_for_context provides both Help and Footer labels
        for tab in [
            Tab::Monitors,
            Tab::Limits,
            Tab::Location,
            Tab::Weather,
            Tab::Settings,
        ] {
            model.active_tab = tab;
            model.input_mode = InputMode::Normal;
            let ctx_cmd = commands_for_workspace(&model);

            // Every command must have non-empty keys, action, and footer_label
            for cmd in ctx_cmd.current_commands {
                assert!(!cmd.keys.is_empty());
                assert!(!cmd.compact_keys.is_empty());
                assert!(!cmd.action.is_empty());
                assert!(!cmd.footer_label.is_empty());
            }

            for cmd in ctx_cmd.global_commands {
                assert!(!cmd.keys.is_empty());
                assert!(!cmd.compact_keys.is_empty());
                assert!(!cmd.action.is_empty());
                assert!(!cmd.footer_label.is_empty());
            }
        }
    }

    fn commands_for_test_context(ctx: UiCommandContext) -> ContextCommands {
        match ctx {
            UiCommandContext::MonitorsList => commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Monitors;
                m.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::List;
                m
            }),
            UiCommandContext::MonitorsDetail => commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Monitors;
                m.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
                m
            }),
            UiCommandContext::Automation => commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Limits;
                m
            }),
            UiCommandContext::AutomationCurve => commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Limits;
                m.automation_focus = crate::tui::model::AutomationRegionFocus::Curve;
                m
            }),
            UiCommandContext::LocationNav => commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Location;
                m
            }),
            UiCommandContext::WeatherObservational => commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Weather;
                m
            }),
            UiCommandContext::SettingsNav => commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Settings;
                m
            }),
            _ => commands_for_footer(&{
                let mut m = Model::new();
                m.show_help = matches!(ctx, UiCommandContext::HelpModal);
                m
            }),
        }
    }

    #[test]
    fn test_footer_help_consistency_invariant() {
        let contexts = [
            UiCommandContext::MonitorsList,
            UiCommandContext::MonitorsDetail,
            UiCommandContext::MonitorsEdit,
            UiCommandContext::Automation,
            UiCommandContext::AutomationCurve,
            UiCommandContext::AutomationEdit,
            UiCommandContext::LocationNav,
            UiCommandContext::LocationCityEdit,
            UiCommandContext::LocationFieldEdit,
            UiCommandContext::WeatherObservational,
            UiCommandContext::SettingsNav,
            UiCommandContext::SettingsEdit,
            UiCommandContext::HelpModal,
        ];

        let prohibited_terms = [
            "Limits",
            "LIMITS",
            "fine-tuning",
            "fine-tune",
            "fine-adjust",
        ];

        for ctx in contexts {
            let cmds = commands_for_test_context(ctx);
            let mut seen_ids = std::collections::HashSet::new();
            for cmd in cmds.current_commands {
                assert!(!cmd.keys.is_empty(), "keys empty for {:?}", cmd.id);
                assert!(
                    !cmd.compact_keys.is_empty(),
                    "compact_keys empty for {:?}",
                    cmd.id
                );
                assert!(!cmd.action.is_empty(), "action empty for {:?}", cmd.id);
                assert!(
                    !cmd.footer_label.is_empty(),
                    "footer_label empty for {:?}",
                    cmd.id
                );
                assert!(
                    cmd.footer_label.chars().count() <= 10,
                    "footer label too long: {}",
                    cmd.footer_label
                );

                for term in prohibited_terms {
                    assert!(
                        !cmd.action.contains(term),
                        "Action contains prohibited term '{}': {}",
                        term,
                        cmd.action
                    );
                    assert!(
                        !cmd.footer_label.contains(term),
                        "Footer label contains prohibited term '{}': {}",
                        term,
                        cmd.footer_label
                    );
                }

                assert!(
                    seen_ids.insert(cmd.id),
                    "Duplicate command ID {:?} in context {:?}",
                    cmd.id,
                    ctx
                );
            }
        }
    }
}
