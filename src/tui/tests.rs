use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::model::{
    ActionKind, ActionState, ActiveModal, ErrorCategory, InputMode, MonitorDiscoveryState,
    MonitorWorkspaceState, ResponsiveMode, Tab,
};
use super::update::{self, Message};
use super::worker::IpcEvent;
use super::{ui, Model};
use crate::backends::BackendKind;
use crate::config::Config;
use crate::discovery::{
    BackendStatus, BackendStatusKind, DdcMonitorDiscovery, DiscoveryBackends, DiscoveryReport,
    DiscoverySummary, PhysicalIdentityStatus,
};
use crate::ipc::{MonitorStatus, StatusResponse};
use crate::policy::{AutomationMilestone, MonitorMilestone, MonitorMilestoneSchedule};
use chrono::{DateTime, FixedOffset, TimeZone};

fn dummy_status(monitor_count: usize) -> StatusResponse {
    let monitors = (0..monitor_count)
        .map(|i| MonitorStatus {
            logical_id: format!("mon-{i}"),
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

fn named_monitor_config(
    logical_id: &str,
    display_name: &str,
    min_pct: u8,
    max_pct: u8,
) -> crate::config::MonitorConfig {
    crate::config::MonitorConfig {
        logical_id: logical_id.to_owned(),
        min_pct,
        max_pct,
        selector: crate::config::MonitorSelector {
            model: Some(display_name.to_owned()),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn configure_monitor_fixture(model: &mut Model, monitors: Vec<crate::config::MonitorConfig>) {
    model.config = Config::default();
    model.config.monitors = monitors;
    model.form = super::form::FormState::new(&model.config);
    model.selected_monitor = 0;
    model.selected_monitor_id = None;
}

fn discovery_report_with_ddc_monitors(monitor_count: usize) -> DiscoveryReport {
    let ddc_monitors = (0..monitor_count)
        .map(|index| DdcMonitorDiscovery {
            occurrence_id: format!("ddc-occurrence:{index}"),
            stable_id: format!("ddc:fixture:{index}"),
            identity_status: PhysicalIdentityStatus::Unique,
            manufacturer: Some(if index == 0 {
                String::from("XMI")
            } else {
                String::from("LEN")
            }),
            model: Some(if index == 0 {
                String::from("Mi Monitor")
            } else {
                String::from("LEN P24h-20")
            }),
            serial: Some(format!("TEST-{index}")),
            display_number: index as u32 + 1,
            bus_number: Some(index as u32 + 7),
            connector: Some(format!("card1-DP-{}", index + 1)),
            brightness_vcp_supported: Some(true),
            backend_viable: true,
            note: None,
        })
        .collect::<Vec<_>>();
    let backend = BackendStatus {
        backend: String::from("fixture"),
        status: BackendStatusKind::Ok,
        available: true,
        message: String::new(),
        guidance: None,
    };

    DiscoveryReport {
        summary: DiscoverySummary {
            ddc_monitors: monitor_count,
            backlight_devices: 0,
            viable_targets: monitor_count,
        },
        backends: DiscoveryBackends {
            ddcutil: backend.clone(),
            brightnessctl: backend.clone(),
            sysfs: backend,
        },
        ddc_observation_complete: true,
        ddc_monitors,
        backlight_devices: Vec::new(),
        windows_displays: Vec::new(),
        notes: Vec::new(),
        config_snippet: String::new(),
    }
}

fn reset_to_empty_monitor_config(model: &mut Model) {
    model.config = Config::default();
    model.config.monitors.clear();
    model.form = super::form::FormState::new(&model.config);
    model.selected_monitor = 0;
    model.selected_monitor_id = None;
}

fn key_event(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind,
        state: KeyEventState::empty(),
    }
}

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

#[test]
fn test_monitor_navigation_zero_monitors() {
    let mut model = Model::new();
    reset_to_empty_monitor_config(&mut model);
    model.status = Some(dummy_status(0));

    model.clamp_monitor_selection();
    assert_eq!(model.selected_monitor, 0);
    assert_eq!(model.monitor_list_state.selected(), None);

    model.move_selection_down();
    assert_eq!(model.selected_monitor, 0);

    model.move_selection_up();
    assert_eq!(model.selected_monitor, 0);

    model.page_down();
    assert_eq!(model.selected_monitor, 0);

    model.page_up();
    assert_eq!(model.selected_monitor, 0);

    model.move_to_first();
    assert_eq!(model.selected_monitor, 0);

    model.move_to_last();
    assert_eq!(model.selected_monitor, 0);
}

#[test]
fn test_monitor_navigation_single_monitor() {
    let mut model = Model::new();
    model.status = Some(dummy_status(1));

    model.clamp_monitor_selection();
    assert_eq!(model.selected_monitor, 0);
    assert_eq!(model.monitor_list_state.selected(), Some(0));

    model.move_selection_down();
    assert_eq!(model.selected_monitor, 0);

    model.move_selection_up();
    assert_eq!(model.selected_monitor, 0);
}

#[test]
fn test_monitor_navigation_multiple_monitors() {
    let mut model = Model::new();
    model.status = Some(dummy_status(10));
    model.clamp_monitor_selection();

    assert_eq!(model.selected_monitor, 0);
    assert_eq!(model.monitor_list_state.selected(), Some(0));

    model.move_selection_down();
    assert_eq!(model.selected_monitor, 1);
    assert_eq!(model.monitor_list_state.selected(), Some(1));

    model.move_selection_up();
    assert_eq!(model.selected_monitor, 0);

    model.page_down();
    assert_eq!(model.selected_monitor, 4);

    model.page_down();
    assert_eq!(model.selected_monitor, 8);

    model.page_down();
    assert_eq!(model.selected_monitor, 9); // clamped to count - 1

    model.page_up();
    assert_eq!(model.selected_monitor, 5);

    model.move_to_last();
    assert_eq!(model.selected_monitor, 9);

    model.move_to_first();
    assert_eq!(model.selected_monitor, 0);
}

#[test]
fn test_monitor_removal_clamps_selection() {
    let mut model = Model::new();
    model.status = Some(dummy_status(6));
    model.selected_monitor = 5;
    model.clamp_monitor_selection();
    assert_eq!(model.selected_monitor, 5);

    // Remove monitors down to 2
    model.status = Some(dummy_status(2));
    model.clamp_monitor_selection();
    assert_eq!(model.selected_monitor, 1);
    assert_eq!(model.monitor_list_state.selected(), Some(1));
}

#[test]
fn test_settings_scrolling_and_navigation() {
    let mut model = Model::new();
    model.active_tab = Tab::Limits;
    // Set 4 monitors in config so Limits has 1 + 4*2 = 9 fields
    let mut cfg = Config::default();
    cfg.monitors.clear();
    for i in 0..4 {
        cfg.monitors.push(crate::config::MonitorConfig {
            logical_id: format!("mon-{i}"),
            ..Default::default()
        });
    }
    model.config = cfg;
    model.form = super::form::FormState::new(&model.config);
    model.active_setting = 0;

    let max_idx = model.tab_field_count(Tab::Limits) - 1;
    assert_eq!(max_idx, 8);

    model.move_selection_down();
    assert_eq!(model.active_setting, 1);

    model.page_down();
    assert_eq!(model.active_setting, 5);

    model.page_down();
    assert_eq!(model.active_setting, 8); // clamped

    model.move_selection_down();
    assert_eq!(model.active_setting, 8); // cannot exceed max

    model.page_up();
    assert_eq!(model.active_setting, 4);

    model.move_to_first();
    assert_eq!(model.active_setting, 0);

    model.move_to_last();
    assert_eq!(model.active_setting, 8);
}

#[test]
fn test_help_modal_scrolling() {
    let mut model = Model::new();
    model.show_help = true;
    model.help_scroll = 0;

    model.scroll_help_down(2, 20);
    assert_eq!(model.help_scroll, 2);

    model.scroll_help_down(30, 20);
    assert_eq!(model.help_scroll, 20); // clamped to max_scroll

    model.scroll_help_up(5);
    assert_eq!(model.help_scroll, 15);

    model.scroll_help_up(25);
    assert_eq!(model.help_scroll, 0); // saturating sub

    model.scroll_help_end(20);
    assert_eq!(model.help_scroll, 20);

    model.scroll_help_home();
    assert_eq!(model.help_scroll, 0);
}

#[test]
fn test_help_modal_key_consumption() {
    let mut model = Model::new();
    model.show_help = true;
    model.active_tab = Tab::Monitors;
    model.help_scroll = 0;

    // Pressing 'j' scrolls help down
    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Char('j'),
            KeyModifiers::empty(),
            KeyEventKind::Press,
        )),
    );
    assert_eq!(model.help_scroll, 1);

    // Numbered key '2' while help is open does NOT switch tabs
    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Char('2'),
            KeyModifiers::empty(),
            KeyEventKind::Press,
        )),
    );
    assert_eq!(model.active_tab, Tab::Monitors);

    // Esc closes help modal
    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Esc,
            KeyModifiers::empty(),
            KeyEventKind::Press,
        )),
    );
    assert!(!model.show_help);
}

#[test]
fn test_key_precedence_ctrl_c() {
    let mut model = Model::new();
    assert!(!model.should_quit);

    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        )),
    );
    assert!(model.should_quit);
}

#[test]
fn test_key_precedence_release_event_ignored() {
    let mut model = Model::new();
    model.active_tab = Tab::Monitors;

    // Release event on '2' does not switch tabs
    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Char('2'),
            KeyModifiers::empty(),
            KeyEventKind::Release,
        )),
    );
    assert_eq!(model.active_tab, Tab::Monitors);
}

#[test]
fn test_numbered_tabs_switch_in_normal_mode() {
    let mut model = Model::new();
    assert_eq!(model.active_tab, Tab::Monitors);

    for (key_char, expected_tab) in [
        ('2', Tab::Limits),
        ('3', Tab::Location),
        ('4', Tab::Weather),
        ('5', Tab::Settings),
        ('1', Tab::Monitors),
    ] {
        update::update(
            &mut model,
            Message::Key(key_event(
                KeyCode::Char(key_char),
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
        );
        assert_eq!(model.active_tab, expected_tab);
    }
}

#[test]
fn test_editing_mode_absorbs_keys() {
    let mut model = Model::new();
    model.active_tab = Tab::Limits;
    model.input_mode = InputMode::Editing;

    // Pressing '1' while editing does not jump to Tab::Monitors
    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Char('1'),
            KeyModifiers::empty(),
            KeyEventKind::Press,
        )),
    );
    assert_eq!(model.active_tab, Tab::Limits);

    // Esc cancels editing
    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Esc,
            KeyModifiers::empty(),
            KeyEventKind::Press,
        )),
    );
    assert_eq!(model.input_mode, InputMode::Normal);
}

#[test]
fn test_theme_modal_consumes_keys() {
    let mut model = Model::new();
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(0));
    model.active_modal = ActiveModal::ThemeSelect(state, crate::tui::theme::Theme::Amber);

    // '2' should not switch tab
    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Char('2'),
            KeyModifiers::empty(),
            KeyEventKind::Press,
        )),
    );
    assert_eq!(model.active_tab, Tab::Monitors);

    // Esc cancels theme modal
    update::update(
        &mut model,
        Message::Key(key_event(
            KeyCode::Esc,
            KeyModifiers::empty(),
            KeyEventKind::Press,
        )),
    );
    assert!(matches!(model.active_modal, ActiveModal::None));
}

#[test]
fn test_responsive_rendering_across_target_dimensions() {
    let tabs = [
        Tab::Monitors,
        Tab::Limits,
        Tab::Location,
        Tab::Weather,
        Tab::Settings,
    ];

    // Targets: 80x24 (Comfortable), 60x20 (Compact), 40x16 (Minimal)
    let sizes = [(80, 24), (60, 20), (40, 16)];

    for (w, h) in sizes {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();

        for tab in tabs {
            let mut model = Model::new();
            model.status = Some(dummy_status(3));
            model.active_tab = tab;

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        }
    }
}

#[test]
fn test_responsive_rendering_too_small_safe_fallback() {
    // Deliberately tiny terminals: 30x8, 20x5, 10x2
    let tiny_sizes = [(30, 8), (20, 5), (10, 2)];

    for (w, h) in tiny_sizes {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(2));

        // Must not panic!
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    }
}

#[test]
fn test_resize_event_handling() {
    let mut model = Model::new();
    model.status = Some(dummy_status(5));
    model.selected_monitor = 4;
    model.clamp_monitor_selection();

    update::update(&mut model, Message::Resize(40, 16));
    assert!(model.selected_monitor <= 4);

    // Simulate monitors dropped to 1 while resized
    model.status = Some(dummy_status(1));
    update::update(&mut model, Message::Resize(60, 20));
    assert_eq!(model.selected_monitor, 0);
}

#[test]
fn test_action_state_suspend_flow_success() {
    let mut model = Model::new();
    model.status = Some(dummy_status(2));

    // User triggers suspend
    model.suspend_writes();
    let ActionState::Pending {
        command_id,
        action: ActionKind::Suspend,
        ..
    } = model.action_state
    else {
        panic!("expected pending suspend");
    };

    // Daemon acknowledges
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandSucceeded {
            command_id,
            action: ActionKind::Suspend,
            message: String::from("suspended writes for 10 minutes"),
        }),
    );

    assert!(matches!(
        model.action_state,
        ActionState::Success {
            action: ActionKind::Suspend,
            ..
        }
    ));

    // Authoritative status arrives from daemon
    let mut updated_status = dummy_status(2);
    updated_status.suspended = true;
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::Status(Box::new(updated_status))),
    );

    assert!(model.status.as_ref().unwrap().suspended);
}

#[test]
fn test_action_state_resume_flow_success() {
    let mut model = Model::new();
    let mut status = dummy_status(2);
    status.suspended = true;
    model.status = Some(status);

    // User triggers resume
    model.resume_writes();
    let ActionState::Pending {
        command_id,
        action: ActionKind::Resume,
        ..
    } = model.action_state
    else {
        panic!("expected pending resume");
    };

    // Daemon acknowledges
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandSucceeded {
            command_id,
            action: ActionKind::Resume,
            message: String::from("resumed automatic apply"),
        }),
    );

    assert!(matches!(
        model.action_state,
        ActionState::Success {
            action: ActionKind::Resume,
            ..
        }
    ));

    // Authoritative status arrives from daemon
    let mut updated_status = dummy_status(2);
    updated_status.suspended = false;
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::Status(Box::new(updated_status))),
    );

    assert!(!model.status.as_ref().unwrap().suspended);
}

#[test]
fn test_action_state_daemon_rejection_leaves_authoritative_state_intact() {
    let mut model = Model::new();
    let initial_status = dummy_status(2);
    model.status = Some(initial_status.clone());

    model.suspend_writes();
    let ActionState::Pending {
        command_id,
        action: ActionKind::Suspend,
        ..
    } = model.action_state
    else {
        panic!("expected pending suspend");
    };

    // Daemon rejects operation
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandFailed {
            command_id,
            action: ActionKind::Suspend,
            error_category: ErrorCategory::DaemonRejected,
            message: String::from("permission denied"),
        }),
    );

    assert!(matches!(
        model.action_state,
        ActionState::Error {
            action: ActionKind::Suspend,
            category: ErrorCategory::DaemonRejected,
            ..
        }
    ));

    // Authoritative state was NOT falsely modified to suspended!
    assert!(!model.status.as_ref().unwrap().suspended);
    assert_eq!(model.status, Some(initial_status));
}

#[test]
fn test_action_state_daemon_unavailable() {
    let mut model = Model::new();

    model.resume_writes();
    let ActionState::Pending {
        command_id,
        action: ActionKind::Resume,
        ..
    } = model.action_state
    else {
        panic!("expected pending resume");
    };

    // Daemon is offline / socket unavailable
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandFailed {
            command_id,
            action: ActionKind::Resume,
            error_category: ErrorCategory::DaemonUnavailable,
            message: String::from("daemon socket not found"),
        }),
    );

    assert!(matches!(
        model.action_state,
        ActionState::Error {
            action: ActionKind::Resume,
            category: ErrorCategory::DaemonUnavailable,
            ..
        }
    ));
}

#[test]
fn test_validation_error_does_not_invoke_ipc() {
    let mut model = Model::new();

    // Type invalid zero duration into suspend input
    model.form.suspend_minutes_input = tui_input::Input::new(String::from("0"));

    model.suspend_writes();

    // Action state reflects validation error
    assert!(matches!(
        model.action_state,
        ActionState::Error {
            action: ActionKind::Suspend,
            category: ErrorCategory::Validation,
            ..
        }
    ));
}

#[test]
fn test_duplicate_command_suppression() {
    let mut model = Model::new();

    // First suspend sets Pending
    model.suspend_writes();
    assert!(matches!(
        model.action_state,
        ActionState::Pending {
            action: ActionKind::Suspend,
            ..
        }
    ));

    let initial_pending_time = match &model.action_state {
        ActionState::Pending { started_at, .. } => *started_at,
        _ => unreachable!(),
    };

    // Second suspend is suppressed
    model.suspend_writes();
    let second_pending_time = match &model.action_state {
        ActionState::Pending { started_at, .. } => *started_at,
        _ => unreachable!(),
    };

    assert_eq!(initial_pending_time, second_pending_time);
}

#[test]
fn test_action_state_expiration() {
    let mut model = Model::new();

    // Success expires after 3 seconds
    model.action_state = ActionState::Success {
        action: ActionKind::Suspend,
        message: String::from("Done"),
        completed_at: Instant::now().checked_sub(Duration::from_secs(4)).unwrap(),
    };
    model.check_action_state_expiration();
    assert_eq!(model.action_state, ActionState::Idle);

    // Warning expires after 6 seconds
    model.action_state = ActionState::Warning {
        action: ActionKind::SaveConfig,
        category: ErrorCategory::DaemonUnavailable,
        message: String::from("Saved to disk — daemon offline"),
        completed_at: Instant::now().checked_sub(Duration::from_secs(7)).unwrap(),
    };
    model.check_action_state_expiration();
    assert_eq!(model.action_state, ActionState::Idle);

    // Error expires after 6 seconds
    model.action_state = ActionState::Error {
        action: ActionKind::Suspend,
        category: ErrorCategory::Transport,
        message: String::from("failed"),
        completed_at: Instant::now().checked_sub(Duration::from_secs(7)).unwrap(),
    };
    model.check_action_state_expiration();
    assert_eq!(model.action_state, ActionState::Idle);

    // Pending times out after 8 seconds
    model.action_state = ActionState::Pending {
        command_id: 1,
        action: ActionKind::Suspend,
        description: String::from("Waiting"),
        started_at: Instant::now().checked_sub(Duration::from_secs(9)).unwrap(),
    };
    model.check_action_state_expiration();
    assert!(matches!(
        model.action_state,
        ActionState::Error {
            action: ActionKind::Suspend,
            category: ErrorCategory::Timeout,
            ..
        }
    ));
}

#[test]
fn test_footer_renders_action_states_across_responsive_viewports() {
    let dimensions = [(80, 24), (60, 20), (40, 16)];

    for (w, h) in dimensions {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut model = Model::new();
        model.status = Some(dummy_status(2));

        // Test Pending rendering
        model.action_state = ActionState::Pending {
            command_id: 1,
            action: ActionKind::Suspend,
            description: String::from("Suspending writes…"),
            started_at: Instant::now(),
        };
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

        // Test Success rendering
        model.action_state = ActionState::Success {
            action: ActionKind::Suspend,
            message: String::from("Writes suspended"),
            completed_at: Instant::now(),
        };
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

        // Test Warning rendering (partial success)
        model.action_state = ActionState::Warning {
            action: ActionKind::SaveConfig,
            category: ErrorCategory::DaemonUnavailable,
            message: String::from("Saved to disk — daemon offline"),
            completed_at: Instant::now(),
        };
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

        // Test Error rendering
        model.action_state = ActionState::Error {
            action: ActionKind::SaveConfig,
            category: ErrorCategory::ConfigWrite,
            message: String::from("permission denied"),
            completed_at: Instant::now(),
        };
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    }
}

#[test]
fn test_save_config_validation_failure_surfaces_error_without_claiming_success() {
    let mut model = Model::new();
    model.config_dirty = true;

    // Invalid FPS (out of range 1..=120)
    model.form.fps_input = tui_input::Input::new(String::from("9999"));
    let saved = model.save_config();
    assert!(!saved);
    assert!(matches!(
        model.action_state,
        ActionState::Error {
            action: ActionKind::SaveConfig,
            category: ErrorCategory::Validation,
            ..
        }
    ));

    // Must not claim success or clear dirty flag
    assert!(model.config_dirty);
}

#[test]
fn test_save_config_full_success_flow() {
    let mut model = Model::new();
    model.config_dirty = true;

    // Trigger save
    let saved = model.save_config();
    assert!(saved);
    assert!(!model.config_dirty);

    let ActionState::Pending {
        command_id,
        action: ActionKind::SaveConfig,
        ..
    } = model.action_state
    else {
        panic!("expected pending save_config");
    };

    // Daemon acknowledges reload
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandSucceeded {
            command_id,
            action: ActionKind::SaveConfig,
            message: String::from("reloaded config"),
        }),
    );

    match &model.action_state {
        ActionState::Success { message, .. } => {
            assert_eq!(message, "Settings saved and applied");
        }
        _ => panic!("expected Success action_state"),
    }
}

#[test]
fn test_save_config_persisted_but_daemon_unavailable_yields_warning() {
    let mut model = Model::new();
    model.config_dirty = true;

    let saved = model.save_config();
    assert!(saved);
    assert!(!model.config_dirty);

    let ActionState::Pending {
        command_id,
        action: ActionKind::SaveConfig,
        ..
    } = model.action_state
    else {
        panic!("expected pending save_config");
    };

    // Daemon is offline / unavailable
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandFailed {
            command_id,
            action: ActionKind::SaveConfig,
            error_category: ErrorCategory::DaemonUnavailable,
            message: String::from("daemon socket not found"),
        }),
    );

    // Must be Warning (partial success), NOT Error!
    match &model.action_state {
        ActionState::Warning {
            action,
            category,
            message,
            ..
        } => {
            assert_eq!(*action, ActionKind::SaveConfig);
            assert_eq!(*category, ErrorCategory::DaemonUnavailable);
            assert_eq!(message, "Saved to disk — daemon offline");
        }
        _ => panic!("expected Warning state for persisted config with offline daemon"),
    }
}

#[test]
fn test_save_config_persisted_but_daemon_rejected_yields_warning() {
    let mut model = Model::new();
    model.config_dirty = true;

    let saved = model.save_config();
    assert!(saved);

    let ActionState::Pending {
        command_id,
        action: ActionKind::SaveConfig,
        ..
    } = model.action_state
    else {
        panic!("expected pending save_config");
    };

    // Daemon rejects reload
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandFailed {
            command_id,
            action: ActionKind::SaveConfig,
            error_category: ErrorCategory::DaemonRejected,
            message: String::from("failed to apply monitors"),
        }),
    );

    match &model.action_state {
        ActionState::Warning {
            action,
            category,
            message,
            ..
        } => {
            assert_eq!(*action, ActionKind::SaveConfig);
            assert_eq!(*category, ErrorCategory::DaemonRejected);
            assert!(message.contains("Saved; daemon reload failed"));
        }
        _ => panic!("expected Warning state for persisted config with rejected reload"),
    }
}

#[test]
fn test_save_config_persisted_but_timeout_yields_warning() {
    let mut model = Model::new();
    let command_id = model.send_command(ActionKind::SaveConfig, crate::ipc::Request::ReloadConfig);

    model.action_state = ActionState::Pending {
        command_id,
        action: ActionKind::SaveConfig,
        description: String::from("Saving configuration…"),
        started_at: Instant::now().checked_sub(Duration::from_secs(9)).unwrap(),
    };

    model.check_action_state_expiration();

    // Must be Warning because config is already saved on disk
    match &model.action_state {
        ActionState::Warning {
            action,
            category,
            message,
            ..
        } => {
            assert_eq!(*action, ActionKind::SaveConfig);
            assert_eq!(*category, ErrorCategory::Timeout);
            assert_eq!(message, "Saved — daemon did not respond");
        }
        _ => panic!("expected Warning state for save_config timeout"),
    }
}

#[test]
fn test_command_id_correlation_discards_stale_and_superseded_completions() {
    let mut model = Model::new();

    // Command 1: Suspend
    model.suspend_writes();
    let ActionState::Pending {
        command_id: cmd1_id,
        ..
    } = model.action_state
    else {
        panic!("expected pending");
    };

    // Command 2: Resume (supersedes Command 1)
    model.resume_writes();
    let ActionState::Pending {
        command_id: cmd2_id,
        ..
    } = model.action_state
    else {
        panic!("expected pending");
    };

    assert_ne!(cmd1_id, cmd2_id);

    // Late completion for Command 1 arrives
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandSucceeded {
            command_id: cmd1_id,
            action: ActionKind::Suspend,
            message: String::from("suspended"),
        }),
    );

    // Command 2 must STILL be pending!
    match model.action_state {
        ActionState::Pending {
            command_id,
            action: ActionKind::Resume,
            ..
        } => {
            assert_eq!(command_id, cmd2_id);
        }
        _ => panic!("Command 2 pending state was overwritten by late Command 1 completion!"),
    }

    // Now Command 2 completion arrives
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::CommandSucceeded {
            command_id: cmd2_id,
            action: ActionKind::Resume,
            message: String::from("resumed"),
        }),
    );

    // Now state transitions to Success for Command 2
    match model.action_state {
        ActionState::Success {
            action: ActionKind::Resume,
            ..
        } => {}
        _ => panic!("expected Success for Command 2"),
    }
}

#[test]
fn test_editor_cursor_insert_middle_and_boundary() {
    let mut model = Model::new();
    model.active_tab = Tab::Location;
    model.active_setting = 3; // Timezone field
    model.form.timezone_input = tui_input::Input::default().with_value(String::from("UTC"));

    model.start_editing();
    assert_eq!(model.input_mode, InputMode::Editing);
    // Cursor starts at end
    assert_eq!(model.form.timezone_input.cursor(), 3);

    // Home moves to start
    model.move_cursor_start();
    assert_eq!(model.form.timezone_input.cursor(), 0);

    // Insert at beginning
    model.insert_char_to_active_input('A');
    assert_eq!(model.form.timezone_input.value(), "AUTC");
    assert_eq!(model.form.timezone_input.cursor(), 1);

    // End moves to end
    model.move_cursor_end();
    assert_eq!(model.form.timezone_input.cursor(), 4);

    // Left moves back 1
    model.move_cursor_left();
    assert_eq!(model.form.timezone_input.cursor(), 3);

    // Insert in middle
    model.insert_char_to_active_input('B');
    assert_eq!(model.form.timezone_input.value(), "AUTBC");
    assert_eq!(model.form.timezone_input.cursor(), 4);

    // Backspace deletes 'B'
    model.backspace_active_input();
    assert_eq!(model.form.timezone_input.value(), "AUTC");
    assert_eq!(model.form.timezone_input.cursor(), 3);

    // Delete deletes 'C' at cursor
    model.delete_active_input();
    assert_eq!(model.form.timezone_input.value(), "AUT");
    assert_eq!(model.form.timezone_input.cursor(), 3);

    // Delete at end is a safe no-op
    model.delete_active_input();
    assert_eq!(model.form.timezone_input.value(), "AUT");
    assert_eq!(model.form.timezone_input.cursor(), 3);

    // Home then Backspace at start is a safe no-op
    model.move_cursor_start();
    assert_eq!(model.form.timezone_input.cursor(), 0);
    model.backspace_active_input();
    assert_eq!(model.form.timezone_input.value(), "AUT");
    assert_eq!(model.form.timezone_input.cursor(), 0);
}

#[test]
fn test_editor_unicode_multibyte_correctness() {
    let mut model = Model::new();
    model.active_tab = Tab::Location;
    model.active_setting = 3;

    // "İstanbul" contains 2-byte 'İ' (U+0130)
    model.form.timezone_input = tui_input::Input::default().with_value(String::from("İstanbul"));
    model.start_editing();
    assert_eq!(model.form.timezone_input.cursor(), 8);

    // Move left across Unicode characters
    for _ in 0..7 {
        model.move_cursor_left();
    }
    // Cursor is now at index 1 (between 'İ' and 's')
    assert_eq!(model.form.timezone_input.cursor(), 1);

    // Insert character after 'İ'
    model.insert_char_to_active_input('X');
    assert_eq!(model.form.timezone_input.value(), "İXstanbul");
    assert_eq!(model.form.timezone_input.cursor(), 2);

    // Delete at cursor ('s')
    model.delete_active_input();
    assert_eq!(model.form.timezone_input.value(), "İXtanbul");

    // Backspace deletes 'X'
    model.backspace_active_input();
    assert_eq!(model.form.timezone_input.value(), "İtanbul");
    assert_eq!(model.form.timezone_input.cursor(), 1);

    // Backspace deletes 'İ'
    model.backspace_active_input();
    assert_eq!(model.form.timezone_input.value(), "tanbul");
    assert_eq!(model.form.timezone_input.cursor(), 0);

    // CJK and Umlaut test without panic
    model.form.timezone_input =
        tui_input::Input::default().with_value(String::from("日本/München"));
    model.move_cursor_start();
    model.move_cursor_right(); // after '日'
    model.insert_char_to_active_input('★');
    assert_eq!(model.form.timezone_input.value(), "日★本/München");
}

#[test]
fn test_editor_word_deletion_and_movement() {
    let mut model = Model::new();
    model.active_tab = Tab::Location;
    model.active_setting = 3;

    // Test 1: "Europe/Istanbul"
    model.form.timezone_input =
        tui_input::Input::default().with_value(String::from("Europe/Istanbul"));
    model.start_editing();
    assert_eq!(model.form.timezone_input.cursor(), 15);

    // Ctrl-W deletes "Istanbul"
    model.delete_prev_word_active_input();
    assert_eq!(model.form.timezone_input.value(), "Europe/");
    assert_eq!(model.form.timezone_input.cursor(), 7);

    // Ctrl-W deletes "Europe/"
    model.delete_prev_word_active_input();
    assert_eq!(model.form.timezone_input.value(), "");
    assert_eq!(model.form.timezone_input.cursor(), 0);

    // Test 2: "My Monitor"
    model.form.timezone_input = tui_input::Input::default().with_value(String::from("My Monitor"));
    model.move_cursor_end();
    model.delete_prev_word_active_input();
    assert_eq!(model.form.timezone_input.value(), "My ");

    // Test 3: "foo   bar"
    model.form.timezone_input = tui_input::Input::default().with_value(String::from("foo   bar"));
    model.move_cursor_end();
    model.delete_prev_word_active_input();
    assert_eq!(model.form.timezone_input.value(), "foo   ");

    // Test 4: Word navigation on "192.168.1.1"
    model.form.timezone_input = tui_input::Input::default().with_value(String::from("192.168.1.1"));
    model.move_cursor_end();
    assert_eq!(model.form.timezone_input.cursor(), 11);

    model.move_cursor_prev_word();
    assert_eq!(model.form.timezone_input.cursor(), 10); // start of "1"

    model.move_cursor_prev_word();
    assert_eq!(model.form.timezone_input.cursor(), 8); // start of "1"

    model.move_cursor_prev_word();
    assert_eq!(model.form.timezone_input.cursor(), 4); // start of "168"

    model.move_cursor_prev_word();
    assert_eq!(model.form.timezone_input.cursor(), 0); // start of "192"

    model.move_cursor_next_word();
    assert_eq!(model.form.timezone_input.cursor(), 4);
}

#[test]
fn test_editor_horizontal_viewport_clamping_and_scrolling() {
    use super::ui::settings::compute_horizontal_viewport;

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
fn test_editor_input_precedence_isolates_global_keys() {
    let mut model = Model::new();
    model.active_tab = Tab::Location;
    model.active_setting = 3;
    model.form.timezone_input = tui_input::Input::default().with_value(String::from("UTC"));

    model.start_editing();
    assert_eq!(model.input_mode, InputMode::Editing);

    // Number keys switch tabs in Normal mode, but MUST be consumed as text in Editing mode!
    for digit in ['1', '2', '3', '4', '5'] {
        update::update(
            &mut model,
            Message::Key(KeyEvent::new(KeyCode::Char(digit), KeyModifiers::NONE)),
        );
        // Tab must STILL be Location!
        assert_eq!(model.active_tab, Tab::Location);
    }
    assert_eq!(model.form.timezone_input.value(), "UTC12345");

    // 'q' must NOT quit SunReactor
    update::update(
        &mut model,
        Message::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
    );
    assert_eq!(model.form.timezone_input.value(), "UTC12345q");

    // '?' must NOT open help modal
    update::update(
        &mut model,
        Message::Key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE)),
    );
    assert!(matches!(
        model.active_modal,
        super::model::ActiveModal::None
    ));

    // Up/Down must NOT navigate to different settings while editing
    update::update(
        &mut model,
        Message::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
    );
    assert_eq!(model.active_setting, 3);
    update::update(
        &mut model,
        Message::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
    );
    assert_eq!(model.active_setting, 3);

    // Esc cancels editing and restores original value
    update::update(
        &mut model,
        Message::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
    );
    assert_eq!(model.input_mode, InputMode::Normal);
    assert_eq!(model.form.timezone_input.value(), "UTC");
}

#[test]
fn test_editor_validation_failure_keeps_editing_mode_with_dirty_input() {
    let mut model = Model::new();
    model.active_tab = Tab::Location;
    model.active_setting = 1; // Latitude field
    model.form.lat_input = tui_input::Input::default().with_value(String::new());

    model.start_editing();

    // Type invalid latitude "999.0" (> 90.0)
    for c in "999.0".chars() {
        model.insert_char_to_active_input(c);
    }
    assert_eq!(model.form.lat_input.value(), "999.0");

    // Press Enter to commit
    update::update(
        &mut model,
        Message::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
    );

    // SUT requirement: on validation error, remain in Editing mode so user can correct it!
    assert_eq!(model.input_mode, InputMode::Editing);
    assert_eq!(model.form.lat_input.value(), "999.0");
    assert!(model.config_dirty);
    assert!(matches!(
        model.action_state,
        ActionState::Error {
            category: ErrorCategory::Validation,
            ..
        }
    ));
}

#[test]
fn test_editor_responsive_rendering_across_viewports() {
    let dimensions = [(80, 24), (60, 20), (40, 16)];

    for (w, h) in dimensions {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut model = Model::new();
        model.active_tab = Tab::Location;
        model.active_setting = 3;
        model.form.timezone_input =
            tui_input::Input::default().with_value(String::from("Europe/Istanbul"));

        model.start_editing();
        assert_eq!(model.input_mode, InputMode::Editing);

        // Render editing state
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

        // Move cursor to middle and render
        model.move_cursor_left();
        model.move_cursor_left();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    }
}

fn find_in_buffer<'a>(
    buffer: &'a ratatui::buffer::Buffer,
    text: &str,
) -> Option<(u16, u16, &'a ratatui::buffer::Cell)> {
    for y in 0..buffer.area.height {
        let mut row_str = String::with_capacity(buffer.area.width as usize);
        for x in 0..buffer.area.width {
            row_str.push_str(buffer.get(x, y).symbol());
        }
        if let Some(idx) = row_str.find(text) {
            let char_idx = row_str[..idx].chars().count() as u16;
            return Some((char_idx, y, buffer.get(char_idx, y)));
        }
    }
    None
}

/// Keeps render-test failures actionable without relying on brittle full
/// snapshots. Individual assertions can include the compact terminal text
/// when a responsive layout moves or removes a specific element.
fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.get(x, y).symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn buffer_row(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer.get(x, y).symbol())
        .collect()
}

#[test]
fn test_all_themes_resolve_palette_and_deserialize() {
    use crate::config::Theme;

    let expected_themes = [
        ("amber", Theme::Amber),
        ("ayudark", Theme::AyuDark),
        ("ayumirage", Theme::AyuMirage),
        ("casiodigital", Theme::CasioDigital),
        ("catppuccinmocha", Theme::CatppuccinMocha),
        ("classicmacintosh", Theme::ClassicMacintosh),
        ("commodore64", Theme::Commodore64),
        ("cyberpunk", Theme::Cyberpunk),
        ("dracula", Theme::Dracula),
        ("everforest", Theme::Everforest),
        ("grayscale", Theme::Grayscale),
        ("gruvbox", Theme::Gruvbox),
        ("hackergreen", Theme::HackerGreen),
        ("kanagawa", Theme::Kanagawa),
        ("materialocean", Theme::MaterialOcean),
        ("monokai", Theme::Monokai),
        ("nightowl", Theme::NightOwl),
        ("nord", Theme::Nord),
        ("nothing", Theme::Nothing),
        ("onedark", Theme::OneDark),
        ("phosphorblue", Theme::PhosphorBlue),
        ("rosepine", Theme::RosePine),
        ("solarizeddark", Theme::SolarizedDark),
        ("synthwave84", Theme::Synthwave84),
        ("terminal", Theme::Terminal),
        ("thinkpad", Theme::ThinkPad),
        ("tokyonight", Theme::TokyoNight),
        ("zenburn", Theme::Zenburn),
    ];

    assert_eq!(Theme::ALL.len(), 28);
    assert_eq!(expected_themes.len(), 28);

    for (key, expected_theme) in expected_themes {
        // Test deserialization from TOML
        let toml_str = format!("theme = \"{key}\"\n");
        let parsed: Result<crate::config::TuiConfig, _> = toml::from_str(&toml_str);
        assert!(
            parsed.is_ok(),
            "Failed to deserialize theme '{key}': {:?}",
            parsed.err()
        );
        assert_eq!(parsed.unwrap().theme, expected_theme);

        // Test palette resolution
        let palette = expected_theme.palette();
        assert_ne!(palette.bg, palette.fg);
        assert!(!expected_theme.name().is_empty());
    }
}

#[test]
fn test_chrome_render_regression_guard() {
    use ratatui::style::Modifier;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.config.tui.theme = crate::config::Theme::Amber;
    model.config.location.city = String::from("Istanbul");
    model.config.location.timezone = String::from("Europe/Istanbul");
    model.motion.masthead_sweep_done = true;

    let palette = model.config.tui.theme.palette();

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // 1. Compact identity keeps the dual-tone Sun/Reactor treatment.
    let (_, _, title_cell) = find_in_buffer(&buffer, "Reactor").expect("header title");
    assert_eq!(title_cell.fg, palette.accent);
    assert!(title_cell.modifier.contains(Modifier::BOLD));
    let (_, _, sun_cell) = find_in_buffer(&buffer, "Sun").expect("Sun prefix rendered");
    assert_eq!(sun_cell.fg, palette.secondary_accent);

    // 2. Live state is a glyph plus a word in the success colour, not a filled badge.
    let (x, y, live_cell) = find_in_buffer(&buffer, "Live").expect("live state");
    assert_eq!(live_cell.fg, palette.success);
    assert_ne!(live_cell.bg, palette.success);
    assert_eq!(buffer.get(x - 2, y).symbol(), "●");

    // 3. Only practical context: applied values, no sun angle, mode, or city.
    assert!(find_in_buffer(&buffer, "25%").is_some());
    assert!(find_in_buffer(&buffer, "Sun +15.0°").is_none());
    assert!(find_in_buffer(&buffer, "MODE:").is_none());
    assert!(find_in_buffer(&buffer, "Istanbul").is_none());

    // 5. Active tab: accent, bold, and an accent rule segment directly below.
    let (tab_x, tab_y, active_tab_cell) =
        find_in_buffer(&buffer, "1 Monitors").expect("active tab");
    assert_eq!(active_tab_cell.fg, palette.accent);
    assert_ne!(active_tab_cell.bg, palette.accent);
    assert!(active_tab_cell.modifier.contains(Modifier::BOLD));
    let indicator = buffer.get(tab_x, tab_y + 1);
    assert_eq!(indicator.symbol(), "━");
    assert_eq!(indicator.fg, palette.accent);
    assert!(find_in_buffer(&buffer, "[1 MONITORS]").is_none());

    // 6. Inactive tabs are muted.
    let (_, _, inactive_tab_cell) = find_in_buffer(&buffer, "2 Automation").expect("tab");
    assert_ne!(inactive_tab_cell.fg, palette.accent);

    // 7. Footer: muted key descriptions on the last row, no mode chip in navigation.
    let (_, hint_y, hint_cell) = find_in_buffer(&buffer, "Select").expect("footer hint");
    assert_eq!(hint_cell.fg, palette.text_muted);
    assert_eq!(hint_y, 23);
    assert!(find_in_buffer(&buffer, "NAV").is_none());

    // 8. Warning feedback replaces the footer with a warning-coloured message.
    model.action_state = ActionState::Warning {
        action: ActionKind::SaveConfig,
        category: ErrorCategory::DaemonUnavailable,
        message: String::from("Saved to disk — daemon offline"),
        completed_at: Instant::now(),
    };
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let warn_buffer = terminal.backend().buffer().clone();
    let (_, warn_y, warn_cell) =
        find_in_buffer(&warn_buffer, "Saved to disk — daemon offline").expect("warning");
    assert_eq!(warn_cell.fg, palette.warning);
    assert!(warn_cell.modifier.contains(Modifier::BOLD));
    assert_eq!(warn_y, 23);

    // 9. Error feedback uses the error colour and a category prefix.
    model.action_state = ActionState::Error {
        action: ActionKind::SaveConfig,
        category: ErrorCategory::ConfigWrite,
        message: String::from("Permission denied"),
        completed_at: Instant::now(),
    };
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let err_buffer = terminal.backend().buffer().clone();
    let (_, _, err_cell) =
        find_in_buffer(&err_buffer, "Save failed: Permission denied").expect("error");
    assert_eq!(err_cell.fg, palette.error);
    assert!(err_cell.modifier.contains(Modifier::BOLD));

    // 10. Offline daemon state.
    model.action_state = ActionState::Idle;
    model.daemon_connection = super::DaemonConnection::Disconnected;
    model.status = None;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let offline_buffer = terminal.backend().buffer().clone();
    let (_, _, offline_cell) = find_in_buffer(&offline_buffer, "Offline").expect("offline");
    assert_eq!(offline_cell.fg, palette.error);
    assert!(offline_cell.modifier.contains(Modifier::BOLD));
}

#[test]
fn test_semantic_styles_chrome_slice_direct_render() {
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier, Style};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.motion.masthead_sweep_done = true;

    // Create custom semantic styles with distinguishable overrides
    let base_palette = crate::config::Theme::Amber.palette();
    let mut custom_styles = base_palette.styles();
    custom_styles.chrome_title = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::UNDERLINED);
    custom_styles.tab_active = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    custom_styles.key_hint_desc = Style::default().fg(Color::LightGreen);

    terminal
        .draw(|f| {
            // Render individual chrome components directly using the semantic styles slice
            super::ui::chrome::render_header(f, &model, Rect::new(0, 0, 80, 3), &custom_styles);
            super::ui::chrome::render_tabs(f, &model, Rect::new(0, 3, 80, 3), &custom_styles);
            super::ui::chrome::render_footer(f, &model, Rect::new(0, 21, 80, 3), &custom_styles);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();

    // 1. Direct header render consumed custom_styles.chrome_title on Reactor.
    let title_match = find_in_buffer(&buffer, "Reactor");
    assert!(title_match.is_some(), "Header title not found");
    let (_, _, title_cell) = title_match.unwrap();
    assert_eq!(title_cell.fg, Color::Cyan);
    assert!(title_cell.modifier.contains(Modifier::UNDERLINED));

    // 2. Direct tabs render consumed custom_styles.tab_active
    let tab_match = find_in_buffer(&buffer, "1 Monitors");
    assert!(tab_match.is_some(), "Active tab not found");
    let (_, _, tab_cell) = tab_match.unwrap();
    assert_eq!(tab_cell.fg, Color::Yellow);
    assert!(tab_cell.modifier.contains(Modifier::BOLD));

    // 3. Direct footer render consumed custom_styles.key_hint_desc
    let footer_match = find_in_buffer(&buffer, "Select");
    assert!(footer_match.is_some(), "Footer hint not found");
    let (_, _, footer_cell) = footer_match.unwrap();
    assert_eq!(footer_cell.fg, Color::LightGreen);
}

#[test]
fn test_chrome_compact_responsive_mode() {
    let backend = TestBackend::new(60, 18);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // 1. Compact header contains identity and badge
    assert!(find_in_buffer(&buffer, "SunReactor").is_some());
    assert!(find_in_buffer(&buffer, "Live").is_some());

    // 2. Compact tabs use short titles; the rule marks the active one.
    assert!(find_in_buffer(&buffer, "1 Mon").is_some());
    assert!(find_in_buffer(&buffer, "2 Auto").is_some());
    assert!(find_in_buffer(&buffer, "━").is_some());

    // 3. Compact footer keeps its commands readable.
    assert!(find_in_buffer(&buffer, "Select").is_some());
    assert!(find_in_buffer(&buffer, "Susp/Res").is_some());
}

#[test]
fn test_chrome_editing_mode_footer() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;
    model.start_editing();

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Footer shows EDIT mode tag and editing-specific commands
    assert!(find_in_buffer(&buffer, "EDIT").is_some());
    assert!(find_in_buffer(&buffer, "Move").is_some());
    assert!(find_in_buffer(&buffer, "Save").is_some());
    assert!(find_in_buffer(&buffer, "Cancel").is_some());
}

#[test]
fn test_chrome_automation_mode_footer() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;

    // Automation opens on the curve control.
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer, "Monitor").is_some());
    assert!(find_in_buffer(&buffer, "Gamma").is_some());
    assert!(find_in_buffer(&buffer, "Schedule").is_some());
    assert!(find_in_buffer(&buffer, "±1 min").is_none());

    // The schedule advertises only milestone commands.
    model.automation_focus = super::model::AutomationRegionFocus::Milestones;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer, "±1 min").is_some());
    assert!(find_in_buffer(&buffer, "Reset").is_some());
    assert!(find_in_buffer(&buffer, "Left/Right change monitor").is_none());
}

#[test]
fn test_settings_footer_compacts_before_clipping_at_comfortable_boundary() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut model = Model::new();
    model.active_tab = Tab::Settings;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    assert!(find_in_buffer(&buffer, "? Help").is_some());
    assert!(find_in_buffer(&buffer, "q Quit").is_some());
}

#[test]
fn test_chrome_non_amber_palettes() {
    use ratatui::style::Modifier;

    for theme in [
        crate::config::Theme::Nord,
        crate::config::Theme::TokyoNight,
        crate::config::Theme::HackerGreen,
        crate::config::Theme::Commodore64,
        crate::config::Theme::Grayscale,
    ] {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.theme = theme;
        model.status = Some(dummy_status(1));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.motion.masthead_sweep_done = true;

        let palette = theme.palette();

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // 1. Product title renders with the primary theme accent.
        let title_match = find_in_buffer(&buffer, "Reactor");
        assert!(title_match.is_some(), "Title missing in theme {theme:?}");
        let (_, _, title_cell) = title_match.unwrap();
        assert_eq!(title_cell.fg, palette.accent);

        // 2. Active tab renders with accent fg
        let tab_match = find_in_buffer(&buffer, "1 Monitors");
        assert!(tab_match.is_some(), "Active tab missing in theme {theme:?}");
        let (_, _, tab_cell) = tab_match.unwrap();
        assert_eq!(tab_cell.fg, palette.accent);
        assert!(tab_cell.modifier.contains(Modifier::BOLD));

        // 3. Footer commands rendered
        assert!(
            find_in_buffer(&buffer, "? Help").is_some(),
            "Footer help missing in {theme:?}"
        );
    }
}

#[test]
fn test_preflight_operational_mode_precedence() {
    use crate::tui::model::OperationalMode;

    let mut model = Model::new();
    let mut status = dummy_status(1);

    // 1. Disconnected -> Offline
    model.daemon_connection = super::DaemonConnection::Disconnected;
    model.status = Some(status.clone());
    assert_eq!(model.operational_mode(), OperationalMode::Offline);

    // 2. Connected but daemon_alive == false -> Offline
    model.daemon_connection = super::DaemonConnection::Connected;
    status.daemon_alive = false;
    model.status = Some(status.clone());
    assert_eq!(model.operational_mode(), OperationalMode::Offline);

    // 3. Alive + Suspended -> Suspended (even if idle_dimmed and override are true)
    status.daemon_alive = true;
    status.suspended = true;
    status.desktop_idle_dimmed = true;
    status.manual_override_active = true;
    model.status = Some(status.clone());
    assert_eq!(model.operational_mode(), OperationalMode::Suspended);

    // 4. Alive + Not Suspended + IdleDimmed -> IdleDimmed (even if override is true)
    status.suspended = false;
    status.desktop_idle_dimmed = true;
    status.manual_override_active = true;
    model.status = Some(status.clone());
    assert_eq!(model.operational_mode(), OperationalMode::IdleDimmed);

    // 5. Alive + Not Suspended + Not Idle + Override -> Override
    status.desktop_idle_dimmed = false;
    status.manual_override_active = true;
    model.status = Some(status.clone());
    assert_eq!(model.operational_mode(), OperationalMode::Override);

    // 6. Normal curve -> Automatic
    status.manual_override_active = false;
    model.status = Some(status);
    assert_eq!(model.operational_mode(), OperationalMode::Automatic);
}

#[test]
fn test_preflight_next_automation_event_calculations() {
    use crate::policy::{AutomationMilestone, MonitorMilestone, MonitorMilestoneSchedule};
    use chrono::{DateTime, FixedOffset, TimeZone};

    let mut model = Model::new();
    let offset = FixedOffset::east_opt(3 * 3600).unwrap();

    let make_time = |h: u32, m: u32| -> DateTime<FixedOffset> {
        offset
            .with_ymd_and_hms(2026, 9, 10, h, m, 0)
            .single()
            .unwrap()
    };

    let milestones = vec![
        MonitorMilestone {
            milestone: AutomationMilestone::RiseStart,
            base_time_local: make_time(6, 0),
            adjusted_time_local: make_time(6, 0),
            target_percent: 7,
            minutes_offset: 0,
        },
        MonitorMilestone {
            milestone: AutomationMilestone::Rise25,
            base_time_local: make_time(7, 0),
            adjusted_time_local: make_time(7, 15), // manual offset +15m
            target_percent: 25,
            minutes_offset: 15,
        },
        MonitorMilestone {
            milestone: AutomationMilestone::Peak,
            base_time_local: make_time(12, 0),
            adjusted_time_local: make_time(12, 0),
            target_percent: 60,
            minutes_offset: 0,
        },
        MonitorMilestone {
            milestone: AutomationMilestone::NightFloor,
            base_time_local: make_time(21, 0),
            adjusted_time_local: make_time(21, 0),
            target_percent: 7,
            minutes_offset: 0,
        },
    ];

    model.monitor_milestones = vec![MonitorMilestoneSchedule {
        logical_id: String::from("mon-0"),
        milestones,
    }];

    // Case 1: Before first milestone (04:00) -> Next is RiseStart at 06:00
    let next_dawn = model
        .next_automation_milestone(&make_time(4, 0))
        .expect("next milestone found");
    assert_eq!(next_dawn.milestone_label, "Rise Start");
    assert_eq!(next_dawn.time_str, "06:00");
    assert_eq!(next_dawn.target_percent, 7);
    assert!(!next_dawn.is_tomorrow);

    // Case 2: Between RiseStart and Rise25 with offset (06:30) -> Next is Rise25 at 07:15 with +15m offset
    let next_sunrise = model
        .next_automation_milestone(&make_time(6, 30))
        .expect("next milestone found");
    assert_eq!(next_sunrise.milestone_label, "Rise 25%");
    assert_eq!(next_sunrise.time_str, "07:15");
    assert_eq!(next_sunrise.minutes_offset, 15);
    assert_eq!(next_sunrise.target_percent, 25);
    assert!(!next_sunrise.is_tomorrow);

    // Case 3: Exactly at milestone (07:15) -> Next is Peak at 12:00
    let next_peak = model
        .next_automation_milestone(&make_time(7, 15))
        .expect("next milestone found");
    assert_eq!(next_peak.milestone_label, "Peak");
    assert_eq!(next_peak.time_str, "12:00");
    assert_eq!(next_peak.target_percent, 60);

    // Case 4: After final milestone (22:00) -> Rollover to tomorrow RiseStart
    let next_tomorrow = model
        .next_automation_milestone(&make_time(22, 0))
        .expect("next milestone found");
    assert_eq!(next_tomorrow.milestone_label, "Rise Start");
    assert_eq!(next_tomorrow.time_str, "06:00");
    assert!(next_tomorrow.is_tomorrow);
}

#[test]
fn test_monitors_workspace_comfortable_multiple_monitors() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![
            named_monitor_config("mon-0", "Mi Monitor", 5, 60),
            named_monitor_config("mon-1", "LEN P24h-20", 8, 90),
        ],
    );
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.selected_monitor = 0;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // 1. One selector region owns selection, with friendly names and a count.
    assert!(find_in_buffer(&buffer, "Monitors").is_some());
    assert!(find_in_buffer(&buffer, "Mi Monitor").is_some());
    assert!(find_in_buffer(&buffer, "Lenovo P24h-20").is_some());
    assert!(find_in_buffer(&buffer, "1 of 2").is_some());
    assert!(find_in_buffer(&buffer, "❯").is_some());

    // 2. Detail sections for the selected monitor.
    assert!(find_in_buffer(&buffer, "Brightness").is_some());
    assert!(find_in_buffer(&buffer, "Brightness range").is_some());
    assert!(find_in_buffer(&buffer, "Automation").is_some());
    assert!(find_in_buffer(&buffer, "DDC").is_some());
    assert!(find_in_buffer(&buffer, "Inspect & fine-tune").is_none());
}

#[test]
fn test_monitors_workspace_selected_vs_unselected_marker() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.selected_monitor = 0;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Selected monitor has the ❯ cursor with focus_marker style (accent fg)
    let marker_match = find_in_buffer(&buffer, "❯");
    assert!(marker_match.is_some());
    let (_, _, marker_cell) = marker_match.unwrap();
    let palette = model.config.tui.theme.palette();
    assert_eq!(marker_cell.fg, palette.accent);
}

#[test]
fn test_monitors_workspace_long_name_truncation_no_panic() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    let mut status = dummy_status(1);
    status.monitors[0].logical_id =
        String::from("very-long-monitor-identifier-that-exceeds-column-width-abcdef-123456");
    model.status = Some(status);
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;

    // Must not panic on long UTF-8 strings
    let res = terminal.draw(|f| ui::ui(f, &mut model));
    assert!(res.is_ok());
    let buffer = terminal.backend().buffer().clone();

    // Name is cleanly truncated with ellipsis
    assert!(find_in_buffer(&buffer, "…").is_some());
}

#[test]
fn test_monitors_brightness_values_are_factual() {
    for (applied_pct, label) in [(5, "5%"), (44, "44%"), (100, "100%")] {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        let mut status = dummy_status(1);
        status.monitors[0].last_applied_percent = Some(applied_pct);
        model.status = Some(status);
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Brightness").is_some());
        assert!(find_in_buffer(&buffer, "Applied").is_some());
        assert!(
            find_in_buffer(&buffer, label).is_some(),
            "Applied percentage {label} missing from buffer"
        );
        assert!(find_in_buffer(&buffer, "CURRENT BRIGHTNESS").is_none());
    }
}

#[test]
fn test_monitors_override_and_backoff_states() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    let mut status = dummy_status(1);
    status.monitors[0].override_percent = Some(85);
    status.monitors[0].backoff_until_epoch_s = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60,
    );
    model.status = Some(status);
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // 1. Manual override is an operating mode, not part of the measurement.
    assert!(find_in_buffer(&buffer, "Applied").is_some());
    assert!(find_in_buffer(&buffer, "25%").is_some());
    assert!(find_in_buffer(&buffer, "Mode").is_some());
    assert!(find_in_buffer(&buffer, "Manual override · 85%").is_some());

    // 2. Write backoff is visible as monitor state.
    assert!(find_in_buffer(&buffer, "Retrying in").is_some());
}

#[test]
fn test_monitors_workspace_compact_and_minimal_viewports() {
    // Compact: 60x18
    {
        let backend = TestBackend::new(60, 18);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(2));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Monitor").is_some());
        assert!(find_in_buffer(&buffer, "Brightness").is_some());
    }

    // Minimal: 45x14
    {
        let backend = TestBackend::new(45, 14);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
    }
}

#[test]
fn test_monitors_workspace_non_amber_themes() {
    for theme in [
        crate::config::Theme::Nord,
        crate::config::Theme::HackerGreen,
    ] {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.theme = theme;
        model.status = Some(dummy_status(1));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
        let buffer = terminal.backend().buffer().clone();

        let palette = theme.palette();
        let marker = find_in_buffer(&buffer, "❯").expect("focus marker present");
        assert_eq!(marker.2.fg, palette.accent);
    }
}

#[test]
fn test_preflight_unknown_brightness_not_zero() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    let mut status = dummy_status(1);
    status.monitors[0].last_applied_percent = None;
    status.monitors[0].override_percent = None;
    model.status = Some(status);
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // 1. Unknown brightness is explicitly represented, never as false '0%'.
    assert!(find_in_buffer(&buffer, "Applied         Unknown").is_some());
    assert!(find_in_buffer(&buffer, "Applied         ● 0%").is_none());
}

#[test]
fn test_preflight_applied_vs_override_distinction() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    let mut status = dummy_status(1);
    status.monitors[0].last_applied_percent = Some(63);
    status.monitors[0].override_percent = Some(85);
    model.status = Some(status);
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Must distinguish applied 63% from an 85% manual override.
    assert!(find_in_buffer(&buffer, "Applied         ● 63%").is_some());
    assert!(find_in_buffer(&buffer, "Mode            Manual override · 85%").is_some());
    assert!(find_in_buffer(&buffer, "APPLIED 63%").is_none());
}

#[test]
fn test_preflight_backend_casing_ddc_backlight() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    let mut status = dummy_status(1);
    status.monitors[0].backend = crate::backends::BackendKind::Ddc;
    model.status = Some(status);
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Must show uppercase DDC, never Rust Debug 'Ddc'
    assert!(find_in_buffer(&buffer, "DDC").is_some());
}

#[test]
fn test_advanced_milestones_table_structure_comfortable() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;
    model.automation_focus = super::model::AutomationRegionFocus::Milestones;

    // Provide milestone schedule
    let offset = FixedOffset::east_opt(3 * 3600).unwrap();
    let make_time = |h: u32, m: u32| -> DateTime<FixedOffset> {
        offset
            .with_ymd_and_hms(2026, 9, 10, h, m, 0)
            .single()
            .unwrap()
    };
    model.monitor_milestones = vec![MonitorMilestoneSchedule {
        logical_id: String::from("mon-0"),
        milestones: vec![
            MonitorMilestone {
                milestone: AutomationMilestone::RiseStart,
                base_time_local: make_time(6, 11),
                adjusted_time_local: make_time(6, 11),
                target_percent: 7,
                minutes_offset: 0,
            },
            MonitorMilestone {
                milestone: AutomationMilestone::Rise25,
                base_time_local: make_time(7, 56),
                adjusted_time_local: make_time(8, 4),
                target_percent: 34,
                minutes_offset: 8,
            },
        ],
    }];

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // 1. Column headers
    assert!(find_in_buffer(&buffer, "Event").is_some());
    assert!(find_in_buffer(&buffer, "Offset").is_some());
    assert!(find_in_buffer(&buffer, "Time").is_some());
    assert!(find_in_buffer(&buffer, "Target").is_some());

    // 2. Data rows
    assert!(find_in_buffer(&buffer, "Rise Start").is_some());
    assert!(find_in_buffer(&buffer, "06:11").is_some());
    assert!(find_in_buffer(&buffer, "Rise 25%").is_some());
    assert!(find_in_buffer(&buffer, "+8m").is_some());
    assert!(
        find_in_buffer(&buffer, "08:04").is_some(),
        "{}",
        buffer_text(&buffer)
    );
    assert!(find_in_buffer(&buffer, "34%").is_some());

    // 3. The whole schedule is one framed functional region.
    assert!(find_in_buffer(&buffer, "Schedule").is_some());

    // Columns are allocated from display widths with a two-cell gap, so
    // adjacent headers and values can never merge (e.g. "Night Floor20:45").
    assert!(buffer_text(&buffer)
        .lines()
        .any(|line| line.contains("Offset") && line.contains("Time")));
    assert!(!buffer_text(&buffer).contains("Rise Start06:11"));
}

#[test]
fn test_advanced_milestones_selection_and_current_markers() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;

    let now = model.current_local_time();
    let past = now - chrono::Duration::hours(1);
    let future = now + chrono::Duration::hours(1);
    model.monitor_milestones = vec![MonitorMilestoneSchedule {
        logical_id: String::from("mon-0"),
        milestones: vec![
            MonitorMilestone {
                milestone: AutomationMilestone::RiseStart,
                base_time_local: past,
                adjusted_time_local: past, // In past so it is current
                target_percent: 7,
                minutes_offset: 0,
            },
            MonitorMilestone {
                milestone: AutomationMilestone::Rise25,
                base_time_local: future,
                adjusted_time_local: future, // In future
                target_percent: 34,
                minutes_offset: 0,
            },
        ],
    }];

    // Row 0 is both selected and current
    model.selected_monitor_milestone = 0;
    model.automation_focus = super::model::AutomationRegionFocus::Milestones;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    assert!(
        find_in_buffer(&buffer, "▸● Rise Start").is_some(),
        "{}",
        buffer_text(&buffer)
    );

    // Switch selection to Row 1: the cursor and the current anchor separate.
    model.selected_monitor_milestone = 1;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer2 = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer2, "▸  Rise 25%").is_some());
    assert!(find_in_buffer(&buffer2, " ● Rise Start").is_some());
}

#[test]
fn test_advanced_milestones_adjustment_and_reset() {
    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.selected_monitor = 0;
    model.selected_monitor_milestone = 0;
    model.config.monitors = vec![crate::config::MonitorConfig {
        logical_id: String::from("mon-0"),
        ..Default::default()
    }];

    let offset = FixedOffset::east_opt(3 * 3600).unwrap();
    let make_time = |h: u32, m: u32| -> DateTime<FixedOffset> {
        offset
            .with_ymd_and_hms(2026, 9, 10, h, m, 0)
            .single()
            .unwrap()
    };
    model.monitor_milestones = vec![MonitorMilestoneSchedule {
        logical_id: String::from("mon-0"),
        milestones: vec![MonitorMilestone {
            milestone: AutomationMilestone::RiseStart,
            base_time_local: make_time(6, 11),
            adjusted_time_local: make_time(6, 11),
            target_percent: 7,
            minutes_offset: 0,
        }],
    }];

    // 1. Adjust +1m
    super::actions::adjust_selected_monitor_milestone(&mut model, 1);
    let sched = model.selected_monitor_schedule().unwrap();
    assert_eq!(sched.milestones[0].minutes_offset, 1);
    assert_eq!(sched.milestones[0].base_time_local, make_time(6, 11)); // Base unchanged
    assert_eq!(sched.milestones[0].target_percent, 7); // Target unchanged

    // 2. Reset with r
    super::actions::reset_selected_monitor_milestone(&mut model);
    let sched_reset = model.selected_monitor_schedule().unwrap();
    assert_eq!(sched_reset.milestones[0].minutes_offset, 0);
    assert_eq!(
        sched_reset.milestones[0].adjusted_time_local,
        make_time(6, 11)
    );
}

#[test]
fn test_advanced_milestones_responsive_compact_and_minimal() {
    // Compact: 60x18
    {
        let backend = TestBackend::new(60, 18);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Limits;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
    }

    // Minimal: 42x12
    {
        let backend = TestBackend::new(42, 12);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Limits;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
    }
}

#[test]
fn test_advanced_milestones_non_amber_theme() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.config.tui.theme = crate::config::Theme::Nord;
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;

    let res = terminal.draw(|f| ui::ui(f, &mut model));
    assert!(res.is_ok());
}

#[test]
fn test_automation_instrument_smoke_across_all_themes() {
    use ratatui::style::Color;

    for theme in crate::tui::theme::Theme::ALL {
        let mut model = Model::with_environment(super::app::ModelEnvironment::isolated());
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
        );
        model.config.location.city = String::from("Istanbul, TR");
        model.config.location.latitude = 41.0082;
        model.config.location.longitude = 28.9784;
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.config.tui.theme = theme;
        model.status = Some(dummy_status(1));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.automation_focus = super::model::AutomationRegionFocus::Curve;
        model.motion.level = crate::config::MotionLevel::Off;
        model.refresh_monitor_milestones();

        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let text = buffer_text(&buffer);
        assert!(text.contains("Today's light cycle"), "theme {theme:?}");
        assert!(text.contains("Solar noon"), "theme {theme:?}");
        assert!(text.contains("Brightness target"), "theme {theme:?}");
        assert!(text.contains("Day length"), "theme {theme:?}");
        assert!(text.contains("Next event"), "theme {theme:?}");
        assert!(text.contains("Sunrise"), "theme {theme:?}");
        assert!(text.contains("Schedule"), "theme {theme:?}");
        assert!(text.contains("Gamma"), "theme {theme:?}");
        assert!(text.contains("Output"), "theme {theme:?}");
        assert!(
            text.contains("Steady") || text.contains("Brightening") || text.contains("Dimming"),
            "theme {theme:?}"
        );
        assert!(!text.contains("Arc solar time"), "theme {theme:?}");
        assert!(!text.contains("ribbon brighter"), "theme {theme:?}");

        let palette = theme.palette();
        let area = buffer
            .content
            .iter()
            .filter(|cell| cell.symbol() == "█" && cell.fg != Color::Reset)
            .count();
        assert!(area > 20, "theme {theme:?} lost the target area");
        assert!(
            buffer.content.iter().all(|cell| !cell
                .symbol()
                .chars()
                .any(|character| ('\u{2801}'..='\u{28ff}').contains(&character))),
            "theme {theme:?} Automation should not use dot-matrix glyphs"
        );

        let (x, y, cell) = find_in_buffer(&buffer, "0.50").expect("focused Curve value");
        assert_eq!(cell.bg, palette.accent, "theme {theme:?}");
        assert_ne!(cell.bg, cell.fg, "theme {theme:?} focus lost contrast");
        assert_ne!(cell.fg, Color::Reset, "theme {theme:?} reset foreground");
        assert!(x < buffer.area.width && y < buffer.area.height);
    }
}

#[test]
fn test_automation_chart_survives_comfortable_masthead() {
    let mut model = Model::with_environment(super::app::ModelEnvironment::isolated());
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
    );
    model.config.location.city = String::from("Istanbul, TR");
    model.config.location.latitude = 41.0082;
    model.config.location.longitude = 28.9784;
    model.config.location.timezone = String::from("Europe/Istanbul");
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;
    model.motion.level = crate::config::MotionLevel::Off;
    model.refresh_monitor_milestones();

    let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
    terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
    let text = buffer_text(terminal.backend().buffer());
    assert!(text.contains("Today's light cycle"), "{text}");
    assert!(text.contains("Brightness target"), "{text}");
    assert!(text.contains("Sunrise"), "{text}");
    assert!(
        text.contains("Afternoon") || text.contains("Morning") || text.contains("Night"),
        "{text}"
    );
}

#[test]
fn test_automation_target_instrument_never_invents_applied_brightness() {
    let mut model = Model::with_environment(super::app::ModelEnvironment::isolated());
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
    );
    let mut status = dummy_status(1);
    status.monitors[0].last_applied_percent = Some(44);
    model.status = Some(status);
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;
    model.motion.level = crate::config::MotionLevel::Off;
    model.refresh_monitor_milestones();

    let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
    terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
    let known = buffer_text(terminal.backend().buffer());
    // 44 in the three-row digit font: `╷ ╷ ╷ ╷` over `╰─┤ ╰─┤`.
    assert!(known.contains("Output"), "{known}");
    assert!(known.contains("╰─┤ ╰─┤"), "{known}");
    assert!(!known.contains("not written yet"), "{known}");

    model.status.as_mut().expect("fixture status").monitors[0].last_applied_percent = None;
    terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
    let unknown = buffer_text(terminal.backend().buffer());
    assert!(unknown.contains("Target · not written yet"), "{unknown}");
    assert!(!unknown.contains("╰─┤ ╰─┤"), "{unknown}");
}

#[test]
fn test_automation_responsive_priority_drops_instrument_before_identity() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Limits;
    model.motion.level = crate::config::MotionLevel::Off;

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let text = buffer_text(terminal.backend().buffer());
    assert!(
        text.contains("Mi Monitor"),
        "selected monitor must remain readable"
    );
    assert!(text.contains("Gamma"));
    assert!(text.contains("Target now"));
    assert!(text.contains("Schedule"));
    assert!(!text.contains("Today's light cycle"));
}

#[test]
fn test_preflight_constrained_milestone_detection() {
    let base = FixedOffset::east_opt(3 * 3600)
        .unwrap()
        .with_ymd_and_hms(2026, 9, 10, 6, 0, 0)
        .unwrap();

    // 1. Unconstrained positive offset: base 06:00, offset +15m -> adjusted 06:15
    let m1 = MonitorMilestone {
        milestone: AutomationMilestone::RiseStart,
        base_time_local: base,
        adjusted_time_local: base + chrono::Duration::minutes(15),
        target_percent: 7,
        minutes_offset: 15,
    };
    assert!(!m1.is_constrained(15));

    // 2. Unconstrained negative offset: base 06:00, offset -15m -> adjusted 05:45
    let m2 = MonitorMilestone {
        milestone: AutomationMilestone::RiseStart,
        base_time_local: base,
        adjusted_time_local: base - chrono::Duration::minutes(15),
        target_percent: 7,
        minutes_offset: -15,
    };
    assert!(!m2.is_constrained(-15));

    // 3. Constrained by adjacent milestone ordering:
    // Requested offset is +90m (07:30), but adjusted time was clamped to 07:00
    let m3 = MonitorMilestone {
        milestone: AutomationMilestone::RiseStart,
        base_time_local: base,
        adjusted_time_local: base + chrono::Duration::minutes(60),
        target_percent: 7,
        minutes_offset: 60,
    };
    assert!(m3.is_constrained(90));

    // 4. Day boundary constraint:
    // Requested offset is -30m (yesterday 23:30), but clamped to day start 00:00
    let m4 = MonitorMilestone {
        milestone: AutomationMilestone::RiseStart,
        base_time_local: base,
        adjusted_time_local: base - chrono::Duration::minutes(10),
        target_percent: 7,
        minutes_offset: -10,
    };
    assert!(m4.is_constrained(-30));

    // 5. Offset reset:
    // User reset offset to 0, but milestone is clamped forward by another event
    let m5 = MonitorMilestone {
        milestone: AutomationMilestone::RiseStart,
        base_time_local: base,
        adjusted_time_local: base + chrono::Duration::minutes(10),
        target_percent: 7,
        minutes_offset: 10,
    };
    assert!(m5.is_constrained(0));
}

#[test]
fn test_limits_workspace_comfortable_multiple_monitors() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![
            named_monitor_config("mon-0", "Mi Monitor", 5, 60),
            named_monitor_config("mon-1", "Lenovo P24h-20", 5, 60),
        ],
    );
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;

    let offset = FixedOffset::east_opt(3 * 3600).unwrap();
    let make_time = |h: u32, m: u32| -> DateTime<FixedOffset> {
        offset
            .with_ymd_and_hms(2026, 9, 10, h, m, 0)
            .single()
            .unwrap()
    };
    model.monitor_milestones = vec![MonitorMilestoneSchedule {
        logical_id: String::from("mon-0"),
        milestones: vec![
            MonitorMilestone {
                milestone: AutomationMilestone::RiseStart,
                base_time_local: make_time(6, 11),
                adjusted_time_local: make_time(6, 11),
                minutes_offset: 0,
                target_percent: 7,
            },
            MonitorMilestone {
                milestone: AutomationMilestone::Peak,
                base_time_local: make_time(12, 45),
                adjusted_time_local: make_time(12, 45),
                minutes_offset: 0,
                target_percent: 60,
            },
        ],
    }];

    let res = terminal.draw(|f| ui::ui(f, &mut model));
    assert!(res.is_ok());

    let buffer = terminal.backend().buffer().clone();

    // Tab 2 is a full-width Automation workspace with explicit monitor context.
    assert!(find_in_buffer(&buffer, "Automation").is_some());
    assert!(find_in_buffer(&buffer, "Monitor").is_some());
    assert!(find_in_buffer(&buffer, "Gamma").is_some());
    assert!(find_in_buffer(&buffer, "Event").is_some());
    assert!(find_in_buffer(&buffer, "Time").is_some());
    assert!(find_in_buffer(&buffer, "Target").is_some());
}

#[test]
fn monitor_workspace_distinguishes_discovery_and_no_hardware_states() {
    let mut model = Model::new();
    reset_to_empty_monitor_config(&mut model);
    model.status = Some(dummy_status(0));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.monitor_discovery =
        MonitorDiscoveryState::Complete(Box::new(discovery_report_with_ddc_monitors(2)));

    assert_eq!(
        model.monitor_workspace_state(),
        MonitorWorkspaceState::DiscoveredButUnconfigured {
            compatible_count: 2,
            importable_count: 2,
            incomplete_ddc_probe: false,
        }
    );

    let mut monitors_terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    monitors_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let monitors_buffer = monitors_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&monitors_buffer, "2 compatible monitor(s) found").is_some());
    assert!(find_in_buffer(&monitors_buffer, "Mi Monitor").is_some());
    assert!(find_in_buffer(&monitors_buffer, "Add 2 monitor(s)").is_some());

    model.active_tab = Tab::Limits;
    let mut automation_terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    automation_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let automation_buffer = automation_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&automation_buffer, "2 compatible monitor(s) found").is_some());
    assert!(find_in_buffer(&automation_buffer, "Add 2 monitor(s)").is_some());

    model.monitor_discovery =
        MonitorDiscoveryState::Complete(Box::new(discovery_report_with_ddc_monitors(0)));
    assert_eq!(
        model.monitor_workspace_state(),
        MonitorWorkspaceState::NoCompatibleHardware
    );
    model.active_tab = Tab::Monitors;
    let mut no_hardware_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    no_hardware_terminal
        .draw(|f| ui::ui(f, &mut model))
        .unwrap();
    let no_hardware_buffer = no_hardware_terminal.backend().buffer().clone();
    assert!(find_in_buffer(
        &no_hardware_buffer,
        "No compatible monitors were discovered"
    )
    .is_some());
}

#[test]
fn monitor_workspace_distinguishes_daemon_and_configured_unavailable_states() {
    let mut model = Model::new();
    reset_to_empty_monitor_config(&mut model);
    model.daemon_connection = super::DaemonConnection::Disconnected;

    assert_eq!(
        model.monitor_workspace_state(),
        MonitorWorkspaceState::DaemonUnavailable
    );
    let mut offline_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    offline_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let offline_buffer = offline_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&offline_buffer, "Daemon unavailable").is_some());

    let mut unavailable = dummy_status(1);
    unavailable.monitors[0].topology = Some(String::from("temporarily_unavailable"));
    model.status = Some(unavailable);
    model.daemon_connection = super::DaemonConnection::Connected;
    assert_eq!(
        model.monitor_workspace_state(),
        MonitorWorkspaceState::ConfiguredUnavailable
    );
    let mut unavailable_terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    unavailable_terminal
        .draw(|f| ui::ui(f, &mut model))
        .unwrap();
    let unavailable_buffer = unavailable_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&unavailable_buffer, "Unavailable").is_some());
    assert!(find_in_buffer(&unavailable_buffer, "No compatible monitors").is_none());
}

#[test]
fn shared_monitor_selector_uses_friendly_configured_identity_in_both_workspaces() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![
            named_monitor_config("mon-0", "Mi Monitor", 5, 60),
            named_monitor_config("mon-1", "Lenovo P24h-20", 5, 60),
        ],
    );
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;

    let mut monitors_terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    monitors_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let first_buffer = monitors_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&first_buffer, "Mi Monitor").is_some());
    assert!(find_in_buffer(&first_buffer, "1 of 2").is_some());

    update::update(
        &mut model,
        Message::Key(crossterm::event::KeyEvent::from(KeyCode::Down)),
    );
    monitors_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let second_buffer = monitors_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&second_buffer, "Lenovo P24h-20").is_some());
    assert!(find_in_buffer(&second_buffer, "2 of 2").is_some());

    model.active_tab = Tab::Limits;
    model.automation_focus = super::model::AutomationRegionFocus::Curve;
    let mut automation_terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    automation_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let automation_buffer = automation_terminal.backend().buffer().clone();
    // Automation keeps the selected monitor compact but never regresses to a
    // raw logical ID when friendly configuration metadata exists.
    let (_, selector_row, _) = find_in_buffer(&automation_buffer, "Lenovo P24h").expect("selector");
    assert!(buffer_row(&automation_buffer, selector_row).contains("Monitor"));
    assert!(find_in_buffer(&automation_buffer, "2 / 2").is_some());
    assert!(find_in_buffer(&automation_buffer, "mon-1").is_none());
}

#[test]
fn shared_monitor_selector_uses_configured_ids_while_daemon_records_are_empty() {
    let mut model = Model::new();
    reset_to_empty_monitor_config(&mut model);
    model.config.monitors = discovery_report_with_ddc_monitors(2).importable_monitor_configs();
    model.form = super::form::FormState::new(&model.config);
    let first_name = model.config.monitors[0]
        .selector
        .model
        .clone()
        .expect("fixture config has a friendly model name");
    let second_id = model.config.monitors[1].logical_id.clone();
    let second_name = super::ui::monitor_selector::monitor_display_name(&model, &second_id);

    let mut status = dummy_status(0);
    status.configured_monitors = 2;
    model.status = Some(status);
    model.daemon_connection = super::DaemonConnection::Connected;
    model.clamp_monitor_selection();

    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let first_buffer = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&first_buffer, &first_name).is_some());
    assert!(find_in_buffer(&first_buffer, "1 of 2").is_some());

    update::update(
        &mut model,
        Message::Key(crossterm::event::KeyEvent::from(KeyCode::Down)),
    );
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let second_buffer = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&second_buffer, &second_name).is_some());
    assert!(find_in_buffer(&second_buffer, "2 of 2").is_some());
}

#[test]
fn test_monitors_workspace_operating_range_and_editing() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 7, 60)],
    );
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::MonitorPaneFocus::Detail;
    model.monitor_control_index = 0; // Minimum brightness

    // Normal mode: vertical range rows match vertical navigation, and the
    // focused value shows ‹ › because ←/→ adjust it.
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer, "Brightness range").is_some());
    assert!(find_in_buffer(&buffer, "Minimum").is_some());
    assert!(find_in_buffer(&buffer, "Maximum").is_some());
    assert!(find_in_buffer(&buffer, "‹  7%  ›").is_some());
    assert!(find_in_buffer(&buffer, "↑/↓ select handle").is_none());
    assert!(find_in_buffer(&buffer, "Tab switch").is_none());

    // Editing mode: the value opens an editing capsule.
    model.start_editing();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_edit = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_edit, "│ 7 │").is_some());
}

#[test]
fn test_monitors_workspace_selection_persistence_in_detail_focus() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.selected_monitor = 1;
    model.monitor_pane_focus = super::MonitorPaneFocus::Detail;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // With detail focused, the list keeps a retained-selection marker.
    assert!(find_in_buffer(&buffer, "› mon-1").is_some());
    assert!(find_in_buffer(&buffer, "❯ mon-1").is_none());
}

#[test]
fn test_power_management_in_settings() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.active_tab = Tab::Settings;
    model.active_setting = super::model::settings_index::IDLE_DIM;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    assert!(find_in_buffer(&buffer, "Dim when idle").is_some());
    assert!(find_in_buffer(&buffer, "Power").is_some());
}

#[test]
fn test_navigation_monitors_and_automation_shortcuts() {
    let mut model = Model::new();
    model.status = Some(dummy_status(2));
    model.active_tab = Tab::Monitors;
    model.selected_monitor = 1;

    // Press 'a' on Monitors -> switches to Tab::Limits (Automation) preserving selected_monitor
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('a'),
        )),
    );
    assert_eq!(model.active_tab, Tab::Limits);
    assert_eq!(model.selected_monitor, 1);

    // Press '1' -> switches back to Monitors
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('1'),
        )),
    );
    assert_eq!(model.active_tab, Tab::Monitors);
    assert_eq!(model.selected_monitor, 1);

    // Press '2' -> switches to Automation
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('2'),
        )),
    );
    assert_eq!(model.active_tab, Tab::Limits);
    assert_eq!(model.selected_monitor, 1);
}

#[test]
fn test_compact_brand_comfortable_viewport() {
    let backend = TestBackend::new(85, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Compact dual-tone identity leaves the workspace as the primary surface.
    assert!(find_in_buffer(&buffer, "SunReactor").is_some());
    assert!(find_in_buffer(&buffer, "____").is_none());

    // Operational telemetry stays terse.
    assert!(find_in_buffer(&buffer, "Live").is_some());
    assert!(find_in_buffer(&buffer, "MODE:").is_none());
}

#[test]
fn test_limits_workspace_long_and_unicode_names() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.config.monitors = vec![
        crate::config::MonitorConfig {
            logical_id: String::from(
                "very-long-monitor-name-that-exceeds-column-width-and-must-truncate",
            ),
            backend: BackendKind::Ddc,
            min_pct: 10,
            max_pct: 80,
            ..Default::default()
        },
        crate::config::MonitorConfig {
            logical_id: String::from("İstanbul-Ekranı-Çalışma"),
            backend: BackendKind::Backlight,
            min_pct: 5,
            max_pct: 50,
            ..Default::default()
        },
    ];
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Limits;

    let res = terminal.draw(|f| ui::ui(f, &mut model));
    assert!(res.is_ok());
}

#[test]
fn test_location_workspace_comfortable_form_and_globe() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.config.location.city = String::from("Istanbul, TR");
    model.config.location.latitude = 41.01384;
    model.config.location.longitude = 28.94966;
    model.config.location.timezone = String::from("Europe/Istanbul");
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;

    let res = terminal.draw(|f| ui::ui(f, &mut model));
    assert!(res.is_ok());

    let buffer = terminal.backend().buffer().clone();

    // Headers & Fields
    assert!(find_in_buffer(&buffer, "Location").is_some());
    assert!(find_in_buffer(&buffer, "Earth").is_some());
    assert!(find_in_buffer(&buffer, "City").is_some());
    assert!(find_in_buffer(&buffer, "Latitude").is_some());
    assert!(find_in_buffer(&buffer, "Longitude").is_some());
    assert!(find_in_buffer(&buffer, "Timezone").is_some());

    // Hemisphere formatted display values
    assert!(find_in_buffer(&buffer, "41.01384° N").is_some());
    assert!(find_in_buffer(&buffer, "28.94966° E").is_some());
    assert!(find_in_buffer(&buffer, "Europe/Istanbul").is_some());
}

#[test]
fn test_location_globe_metadata_reflows_without_clipping_in_medium_viewport() {
    let backend = TestBackend::new(80, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.config.location.city = String::from("Istanbul, TR");
    model.config.location.latitude = 41.01384;
    model.config.location.longitude = 28.94966;
    model.config.location.timezone = String::from("Europe/Istanbul");
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    assert!(find_in_buffer(&buffer, "Earth").is_some());
    assert!(find_in_buffer(&buffer, "Istanbul, TR").is_some());
    assert!(find_in_buffer(&buffer, "Europe/Istanbul").is_some());
}

#[test]
fn test_location_workspace_western_and_southern_hemispheres() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    // Southern + Eastern (Sydney)
    model.config.location.city = String::from("Sydney, AU");
    model.config.location.latitude = -33.8688;
    model.config.location.longitude = 151.2093;
    model.config.location.timezone = String::from("Australia/Sydney");
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer, "33.86880° S").is_some());
    assert!(find_in_buffer(&buffer, "151.20930° E").is_some());

    // Northern + Western (New York)
    model.config.location.city = String::from("New York, US");
    model.config.location.latitude = 40.7128;
    model.config.location.longitude = -74.0060;
    model.config.location.timezone = String::from("America/New_York");
    model.form.refresh_from_config(&model.config);

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer2 = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer2, "40.71280° N").is_some());
    assert!(find_in_buffer(&buffer2, "74.00600° W").is_some());

    // Extreme boundaries (±90, ±180)
    assert_eq!(
        crate::tui::ui::location::format_latitude_hemisphere(90.0),
        "90.00000° N"
    );
    assert_eq!(
        crate::tui::ui::location::format_latitude_hemisphere(-90.0),
        "90.00000° S"
    );
    assert_eq!(
        crate::tui::ui::location::format_longitude_hemisphere(180.0),
        "180.00000° E"
    );
    assert_eq!(
        crate::tui::ui::location::format_longitude_hemisphere(-180.0),
        "180.00000° W"
    );
}

#[test]
fn test_location_workspace_focused_and_editing_city() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.config.location.city = String::from("Istanbul, TR");
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;
    model.active_setting = 0; // City field focused

    // Normal mode: ❯ cursor on City, committed value shown
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer, "❯ City").is_some());
    assert!(find_in_buffer(&buffer, "Istanbul, TR").is_some());

    // Switch to editing mode with new query "Lond"
    model.input_mode = InputMode::Editing;
    model.form.city_search_input = tui_input::Input::default().with_value(String::from("Lond"));
    model.form.city_search_results = vec![0, 1]; // Mock results
    model.form.city_search_selected_index = 0;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_edit = terminal.backend().buffer().clone();

    // The working buffer has its own editing capsule, not concatenated with "Istanbul".
    assert!(find_in_buffer(&buffer_edit, "│ Lond").is_some());
    assert!(find_in_buffer(&buffer_edit, "Istanbul, TRLond").is_none());
    assert!(find_in_buffer(&buffer_edit, "Matches").is_some());
}

#[test]
fn test_location_workspace_unicode_and_cjk_city_input() {
    let mut model = Model::new();
    model.active_tab = Tab::Location;
    model.input_mode = InputMode::Editing;
    model.active_setting = 0;

    for unicode_city in ["İstanbul", "Zürich", "São Paulo", "Łódź", "東京"] {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        model.form.city_search_input =
            tui_input::Input::default().with_value(String::from(unicode_city));
        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok(), "Failed on city: {unicode_city}");

        let buffer = terminal.backend().buffer().clone();
        if unicode_city == "東京" {
            // In Ratatui, 2-column wide CJK characters occupy 2 cells: (char, continuation space).
            assert!(find_in_buffer(&buffer, "東").is_some());
            assert!(find_in_buffer(&buffer, "京").is_some());
        } else {
            assert!(
                find_in_buffer(&buffer, unicode_city).is_some(),
                "Missing Unicode text {unicode_city} in buffer"
            );
        }
    }
}

#[test]
fn test_location_workspace_responsive_compact_and_minimal() {
    // Compact layout (65x22): stacked form + globe
    {
        let backend = TestBackend::new(65, 22);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.active_tab = Tab::Location;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());

        let buffer = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "Location").is_some());
        assert!(find_in_buffer(&buffer, "Earth").is_some());
    }

    // Minimal layout (50x16): the globe is omitted and the fields stay readable
    {
        let backend = TestBackend::new(50, 16);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.active_tab = Tab::Location;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());

        let buffer = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "Location").is_some());
        assert!(find_in_buffer(&buffer, "Latitude").is_some());
        assert!(find_in_buffer(&buffer, "Earth").is_none());
    }

    // Strict Minimal layout (45x12): map omitted, essential fields fit without panic
    {
        let backend = TestBackend::new(45, 12);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.active_tab = Tab::Location;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());

        let buffer = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "Location").is_some());
        assert!(find_in_buffer(&buffer, "City").is_some());
    }
}

#[test]
fn test_location_workspace_non_amber_themes() {
    for theme in [
        crate::config::Theme::Nord,
        crate::config::Theme::HackerGreen,
    ] {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.theme = theme;
        model.active_tab = Tab::Location;
        model.active_setting = 1; // Latitude focused

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());

        let buffer = terminal.backend().buffer().clone();
        let marker = find_in_buffer(&buffer, "❯").expect("focus marker present");
        assert_eq!(marker.2.fg, theme.palette().accent);
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Phase 5.5.2: Header / Monitors / Location Polish Regression Tests
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_phase5_5_2_compact_header_spacing_and_telemetry() {
    // 1. Comfortable Mode: compact brand plus two rows of useful telemetry.
    {
        let backend = TestBackend::new(85, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul, TR");
        model.daemon_connection = super::DaemonConnection::Connected;
        model.status = Some(dummy_status(1));
        model.active_tab = Tab::Monitors;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Compact product identity remains visible without consuming the workspace.
        assert!(find_in_buffer(&buffer, "SunReactor").is_some());
        assert!(find_in_buffer(&buffer, "____").is_none());

        // Centered telemetry cluster present
        assert!(find_in_buffer(&buffer, "Live").is_some());
        assert!(find_in_buffer(&buffer, "25%").is_some());
        assert!(find_in_buffer(&buffer, "Istanbul, TR").is_none());

        // Redundant MODE: AUTOMATIC removed
        assert!(find_in_buffer(&buffer, "MODE:").is_none());
        assert!(find_in_buffer(&buffer, "MODE: AUTOMATIC").is_none());
    }

    // 2. Compact Mode (65x18): No redundant AUTOMATIC in header
    {
        let backend = TestBackend::new(65, 18);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul");
        model.daemon_connection = super::DaemonConnection::Connected;
        model.status = Some(dummy_status(1));
        model.active_tab = Tab::Location;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "SunReactor").is_some());
        assert!(find_in_buffer(&buffer, "Live").is_some());
        assert!(find_in_buffer(&buffer, "AUTOMATIC").is_none());
    }

    // 3. Minimal Mode (45x12): One-line header
    {
        let backend = TestBackend::new(45, 12);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Location;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "SunReactor").is_some());
        assert!(find_in_buffer(&buffer, "Live").is_some());
        assert!(find_in_buffer(&buffer, "AUTOMATIC").is_none());
    }
}

#[test]
fn test_phase5_5_2_selected_monitor_legibility_and_contrast_across_themes() {
    for theme in [
        crate::config::Theme::Amber,
        crate::config::Theme::Nord,
        crate::config::Theme::HackerGreen,
        crate::config::Theme::TokyoNight,
        crate::config::Theme::Grayscale,
        crate::config::Theme::Commodore64,
    ] {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.theme = theme;
        model.status = Some(dummy_status(2));
        model.daemon_connection = super::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.selected_monitor = 0;
        model.monitor_pane_focus = super::model::MonitorPaneFocus::List;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        let palette = theme.palette();

        // 1. The focused selector uses one structural rail; it does not reuse
        // the Automation table cursor.
        let focused_marker = find_in_buffer(&buffer, "❯").expect("focused selector rail");
        assert_eq!(focused_marker.2.fg, palette.accent);
        assert!(find_in_buffer(&buffer, "▸").is_none());

        // 2. When focus moves to the detail pane, the list keeps a quieter
        // retained-selection glyph so focus and selection never look alike.
        model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer_detail = terminal.backend().buffer().clone();

        let retained_marker = find_in_buffer(&buffer_detail, "› mon-0").expect("retained marker");
        assert_eq!(retained_marker.2.fg, palette.text_muted);
        assert_ne!(retained_marker.2.fg, palette.accent);
    }
}

#[test]
fn test_phase5_5_2_operating_range_and_gamma_distinction() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 7, 60)],
    );
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    model.monitor_control_index = 0; // Minimum brightness

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    assert!(find_in_buffer(&buffer, "Brightness range").is_some());
    assert!(find_in_buffer(&buffer, "Minimum").is_some());
    assert!(find_in_buffer(&buffer, "Maximum").is_some());
    assert!(find_in_buffer(&buffer, "Field").is_some());
    // Display colour gamma terminology never appears.
    assert!(find_in_buffer(&buffer, "Gamma correction").is_none());
    assert!(find_in_buffer(&buffer, "gamma").is_none());

    // The curve parameter is owned and editable in Automation.
    model.active_tab = Tab::Limits;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_auto = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_auto, "Gamma").is_some());

    assert!(find_in_buffer(&buffer, "Inspect & fine-tune").is_none());
    assert!(find_in_buffer(&buffer, "[a]").is_none());

    model.active_tab = Tab::Monitors;
    model.start_editing();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_edit = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_edit, "│ 7 │").is_some());
}

#[test]
fn test_phase5_5_2_city_clean_edit_entry_and_cancel_restore() {
    let mut model = Model::new();
    model.config.location.city = String::from("Istanbul, TR");
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;
    model.active_setting = 0; // City

    assert_eq!(model.form.city_search_input.value(), "Istanbul, TR");

    // Enter edit mode on City: must start with clean empty buffer for searching
    model.start_editing();
    assert_eq!(model.input_mode, InputMode::Editing);
    assert_eq!(model.form.city_search_input.value(), "");
    assert!(model.form.city_search_results.is_empty());

    // User cancels editing: must restore prior committed city
    model.cancel_editing();
    assert_eq!(model.input_mode, InputMode::Normal);
    assert_eq!(model.form.city_search_input.value(), "Istanbul, TR");
    assert_eq!(model.config.location.city, "Istanbul, TR");
}

#[test]
fn test_phase5_5_2_location_globe_projection_and_hierarchy() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.config.location.city = String::from("Tokyo, JP");
    model.config.location.latitude = 35.6895;
    model.config.location.longitude = 139.6917;
    model.config.location.timezone = String::from("Asia/Tokyo");
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Section 1: Location Form
    assert!(find_in_buffer(&buffer, "Location").is_some());
    assert!(find_in_buffer(&buffer, "City").is_some());
    assert!(find_in_buffer(&buffer, "Tokyo, JP").is_some());

    // Coordinates are plain fields in the same form.
    assert!(find_in_buffer(&buffer, "35.68950° N").is_some());
    assert!(find_in_buffer(&buffer, "139.69170° E").is_some());
    assert!(find_in_buffer(&buffer, "Asia/Tokyo").is_some());

    // Globe instrument metadata bar: clean location identity, no theatrical slogan
    assert!(find_in_buffer(&buffer, "DRIVES AUTOMATION").is_none());
    assert!(find_in_buffer(&buffer, "Earth").is_some());
}

// ══════════════════════════════════════════════════════════════════════════
// Phase 5.6: Solar Cockpit Design Language + Kinetic Telemetry Tests
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_phase5_6_motion_architecture_timing_and_phases() {
    let now = Instant::now();
    let duration = Duration::from_millis(500);
    let transient = super::motion::TransientMotion::new(
        super::motion::TransientKind::RangeCommit { min: true },
        now,
        duration,
    );

    // 1. Initial phase at start is 0.0
    assert_eq!(transient.phase(now), Some(0.0));

    // 2. Phase halfway (250ms) is approximately 0.5
    let half = now + Duration::from_millis(250);
    let p_half = transient.phase(half).expect("halfway phase");
    assert!((p_half - 0.5).abs() < 0.02);

    // 3. Phase at exact duration or later self-expires (returns None)
    let end = now + Duration::from_millis(500);
    assert!(transient.phase(end).is_none());
    let past = now + Duration::from_millis(800);
    assert!(transient.phase(past).is_none());

    // 4. MotionLevel policies
    let mut state = super::motion::UiMotionState::new_at(now);
    assert_eq!(state.level, super::motion::MotionLevel::Instrument);

    // Under MotionLevel::Off, triggers do nothing
    state.level = super::motion::MotionLevel::Off;
    state.trigger(
        super::motion::TransientKind::RangeCommit { min: true },
        now,
        duration,
    );
    assert!(state.active_transient.is_none());
    assert!(state.masthead_sweep_phase(now).is_none());
    assert!(state.heartbeat_pulse_phase(now).is_none());

    // Under MotionLevel::Reduced, broad acquisition rings are suppressed
    state.level = super::motion::MotionLevel::Reduced;
    state.trigger(
        super::motion::TransientKind::LocationAcquisition {
            lon: 10.0,
            lat: 20.0,
        },
        now,
        duration,
    );
    assert!(state.active_transient.is_none());
}

#[test]
fn test_phase5_6_brand_sweep_lifecycle() {
    let backend = TestBackend::new(85, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let now = Instant::now();
    let mut model = Model::new();
    model.daemon_connection = super::DaemonConnection::Connected;
    model.status = Some(dummy_status(1));
    model.motion = super::motion::UiMotionState::new_at(now);

    // 1. Startup phase is active
    let p0 = model.motion.masthead_sweep_phase(now);
    assert!(p0.is_some());

    // Render during sweep
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_sweep = terminal.backend().buffer().clone();

    // Compact wordmark stays intact while its primary tone sweeps.
    assert!(find_in_buffer(&buffer_sweep, "SunReactor").is_some());

    // 2. Fast forward past duration: sweep expires
    let past = now + Duration::from_millis(700);
    model.motion.tick(past);
    assert!(model.motion.masthead_sweep_done);
    assert!(model.motion.masthead_sweep_phase(past).is_none());

    // Settled render preserves the compact identity.
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_settled = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_settled, "SunReactor").is_some());
}

#[test]
fn test_phase5_6_live_heartbeat_and_offline_invariance() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let now = Instant::now();
    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.motion = super::motion::UiMotionState::new_at(now);

    let dot_cell = |buffer: &ratatui::buffer::Buffer| {
        let (x, y, _) = find_in_buffer(buffer, "● Live").expect("live state");
        buffer.get(x, y).clone()
    };

    // 0. Stable state before heartbeat
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let cell_stable = dot_cell(terminal.backend().buffer());

    // 1. A real status response pulses the live dot.
    let pulse_now = Instant::now();
    model.motion.record_heartbeat(pulse_now);
    assert!(model.motion.heartbeat_pulse_phase(pulse_now).is_some());
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let cell_pulse = dot_cell(terminal.backend().buffer());
    assert_ne!(cell_stable.fg, cell_pulse.fg);
    assert_eq!(cell_stable.bg, cell_pulse.bg);

    // 2. After the pulse the dot settles back.
    model.motion.last_heartbeat_at = Some(
        Instant::now()
            .checked_sub(Duration::from_millis(500))
            .unwrap(),
    );
    assert!(model.motion.heartbeat_pulse_phase(Instant::now()).is_none());
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let cell_settled = dot_cell(terminal.backend().buffer());
    assert_eq!(cell_settled.fg, cell_stable.fg);

    // 3. Offline is static.
    model.daemon_connection = super::DaemonConnection::Disconnected;
    model.status = None;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_offline = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_offline, "○ Offline").is_some());
}

#[test]
fn test_phase5_6_operating_range_cockpit_track_and_commit_settle() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let now = Instant::now();
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 7, 60)],
    );
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    model.monitor_control_index = 0; // Minimum brightness
    model.motion = super::motion::UiMotionState::new_at(now);

    // 1. Normal state: Cockpit track shows 0..100 boundary with min/max span
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer, "Brightness range").is_some());
    assert!(find_in_buffer(&buffer, "0 ").is_some());
    assert!(find_in_buffer(&buffer, "100").is_some());
    assert!(find_in_buffer(&buffer, "Minimum").is_some());
    assert!(find_in_buffer(&buffer, "Maximum").is_some());

    // 2. Focused MIN emphasizes its vertical field and matching track handle.
    assert!(find_in_buffer(&buffer, "‹  7%  ›").is_some());
    assert!(find_in_buffer(&buffer, "↑/↓ select handle").is_none());

    // 3. Focus MAX (index 1)
    model.monitor_control_index = 1;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_max = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_max, "‹  60%  ›").is_some());
    assert!(find_in_buffer(&buffer_max, "‹  7%  ›").is_none());
}

#[test]
fn test_phase5_6_location_acquisition_ping() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let now = Instant::now();
    let mut model = Model::new();
    model.config.location.city = String::from("Istanbul, TR");
    model.config.location.latitude = 41.0138;
    model.config.location.longitude = 28.9497;
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;
    model.motion = super::motion::UiMotionState::new_at(now);

    // 1. Static state before acquisition
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_static = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_static, "Istanbul, TR").is_some());

    // 2. Acquisition ping triggered upon accepting a new location
    model.motion.trigger(
        super::motion::TransientKind::LocationAcquisition {
            lon: 28.9497,
            lat: 41.0138,
        },
        now,
        Duration::from_millis(650),
    );
    assert!(model.motion.location_acquisition_phase(now).is_some());

    // 3. Render during active acquisition (draws expanding concentric circles)
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

    // 4. Settle after 650ms
    model.motion.tick(now + Duration::from_millis(700));
    assert!(model
        .motion
        .location_acquisition_phase(now + Duration::from_millis(700))
        .is_none());
}

#[test]
fn test_phase5_6_automation_milestone_micro_settle() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    let now = Instant::now();
    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;
    model.motion = super::motion::UiMotionState::new_at(now);

    // 1. Milestone adjustment triggers micro-settle for milestone index 0
    model.motion.trigger(
        super::motion::TransientKind::MilestoneAdjust { index: 0 },
        now,
        Duration::from_millis(250),
    );
    assert!(model.motion.milestone_adjust_phase(now, 0).is_some());
    assert!(model.motion.milestone_adjust_phase(now, 1).is_none());

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

    // 2. After 250ms, micro-settle self-expires
    model.motion.tick(now + Duration::from_millis(300));
    assert!(model
        .motion
        .milestone_adjust_phase(now + Duration::from_millis(300), 0)
        .is_none());
}

// ══════════════════════════════════════════════════════════════════════════
// Phase 7: Settings v2 Tests — Cockpit Systems Panel + Motion Preference
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_phase7_settings_workspace_comfortable_layout() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut model = Model::new();
    model.config.tui.effects = crate::config::MotionLevel::Instrument;
    model.motion.level = crate::config::MotionLevel::Instrument;
    model.active_tab = Tab::Settings;
    model.active_setting = 0; // Theme
    model.daemon_connection = super::DaemonConnection::Connected;
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
fn test_phase7_settings_navigation_and_effects_toggle_live() {
    let mut model = Model::new();
    model.config.tui.effects = crate::config::MotionLevel::Instrument;
    model.motion.level = crate::config::MotionLevel::Instrument;
    model.active_tab = Tab::Settings;
    model.active_setting = 0;
    assert_eq!(model.tab_field_count(Tab::Settings), 10);

    // 1. Navigate down to Effects (index 1)
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
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
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        )),
    );
    assert_eq!(
        model.config.tui.effects,
        crate::config::MotionLevel::Reduced
    );
    assert_eq!(model.motion.level, crate::config::MotionLevel::Reduced);

    // 3. Press Space to cycle to Off
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(' '),
        )),
    );
    assert_eq!(model.config.tui.effects, crate::config::MotionLevel::Off);
    assert_eq!(model.motion.level, crate::config::MotionLevel::Off);
    assert!(model.motion.active_transient.is_none());

    // 4. Press Right to cycle back to Instrument
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Right,
        )),
    );
    assert_eq!(
        model.config.tui.effects,
        crate::config::MotionLevel::Instrument
    );
    assert_eq!(model.motion.level, crate::config::MotionLevel::Instrument);

    // 5. Press Left to cycle backwards to Off
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Left,
        )),
    );
    assert_eq!(model.config.tui.effects, crate::config::MotionLevel::Off);
    assert_eq!(model.motion.level, crate::config::MotionLevel::Off);
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
    model.form.suspend_minutes_input = tui_input::Input::default().with_value(String::from("45"));

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer2 = terminal.backend().buffer().clone();

    assert!(find_in_buffer(&buffer2, "15").is_some());
    assert!(find_in_buffer(&buffer2, "45").is_some());
    assert!(find_in_buffer(&buffer2, "min").is_some());
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
    model.daemon_connection = super::DaemonConnection::Disconnected;
    model.status = None;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_offline = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_offline, "Offline").is_some());
    assert!(find_in_buffer(&buffer_offline, "daemon service unreachable").is_none());

    // 2. Connected and Suspended state
    model.daemon_connection = super::DaemonConnection::Connected;
    let mut status = dummy_status(1);
    status.suspended = true;
    status.suspend_until_epoch_s = Some(1_800_000_000);
    model.status = Some(status);

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_suspended = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_suspended, "Suspended").is_some());

    // 3. Suspend & Resume key triggers on Tab::Settings
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('s'),
        )),
    );
    assert!(matches!(
        model.action_state,
        super::model::ActionState::Pending {
            action: super::model::ActionKind::Suspend,
            ..
        }
    ));

    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('r'),
        )),
    );
    assert!(matches!(
        model.action_state,
        super::model::ActionState::Pending {
            action: super::model::ActionKind::Resume,
            ..
        }
    ));
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
        model.active_setting = super::model::settings_index::REFRESH_RATE;
        model.daemon_connection = super::DaemonConnection::Connected;
        model.status = Some(dummy_status(1));

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Interface").is_some());
        assert!(find_in_buffer(&buffer, "Power").is_some());
        assert!(find_in_buffer(&buffer, "Effects").is_some());
        let (rx, ry, _) = find_in_buffer(&buffer, "Animation rate").expect("focused row visible");
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
        model.daemon_connection = super::DaemonConnection::Connected;
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
        model.daemon_connection = super::DaemonConnection::Connected;
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

fn dummy_weather_status(
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

#[test]
fn test_phase8_weather_states_disabled_and_unavailable() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Explicitly Disabled in config
    {
        let mut model = Model::new();
        model.config.weather.enabled = false;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        model.status = Some(dummy_status(1));

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "╭ Weather").is_some());
        assert!(find_in_buffer(&buffer, "Weather is off").is_some());
        assert!(find_in_buffer(&buffer, "Enable weather in Settings").is_some());
        assert!(
            find_in_buffer(&buffer, "Weather integration is disabled in configuration").is_none()
        );
        assert!(find_in_buffer(&buffer, "ATMOSPHERIC SUBSYSTEM").is_none());
        assert!(find_in_buffer(&buffer, "OpenWeather API Key").is_none());
    }

    // 2. Unavailable: enabled, but no weather data returned yet
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        let mut weather = dummy_weather_status(true, false, false, None, None, 0);
        weather.condition = crate::weather::WeatherCondition::Unknown;
        status.weather = Some(weather);
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "╭ Weather").is_some());
        assert!(find_in_buffer(&buffer, "Loading weather").is_some());
        assert!(find_in_buffer(&buffer, "Contacting OpenWeather").is_some());
    }
}

#[test]
fn test_phase8_weather_states_fresh_stale_and_error_with_cache() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Fresh weather telemetry
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(42),
            Some(0.88),
            8,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
        assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
        assert!(find_in_buffer(&buffer, "Up to date").is_some());
        assert!(find_in_buffer(&buffer, "Current weather").is_none());
        assert!(find_in_buffer(&buffer, "×0.88").is_none());
    }

    // 2. Stale cached data: keeps chart & instruments visible, demoted with ! Stale
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            false,
            true,
            Some(42),
            Some(0.88),
            8,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
        assert!(find_in_buffer(&buffer, "Stale").is_some());
        assert!(find_in_buffer(&buffer, "Dims daylight").is_none());
    }

    // 3. Error with usable cache: keeps cached chart & sky visible with ! Refresh Failed
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.now_epoch_s = 1_700_002_820; // 47m after fetch, within 60m TTL
        let mut ws = dummy_weather_status(true, false, false, Some(42), Some(0.88), 8);
        ws.state = crate::weather::WeatherState::NetworkError;
        ws.last_error = Some(String::from("openweather network timeout"));
        status.weather = Some(ws);
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Weather unavailable").is_some());
        assert!(find_in_buffer(&buffer, "Retry").is_some());
        assert!(find_in_buffer(&buffer, "24 hour forecast").is_none());
    }
}

#[test]
fn test_phase8_weather_extreme_cloud_cover_0_and_100() {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // 0% cloud cover: Clear, 100% nominal curve
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(0),
            Some(1.0),
            8,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(buffer_text(&buffer)
            .lines()
            .any(|line| line.contains("Cloud cover") && line.contains("0%")));
        assert!(find_in_buffer(&buffer, "×1.00").is_none());
    }

    // 100% cloud cover: nominal attenuation
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(100),
            Some(0.50),
            8,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(buffer_text(&buffer)
            .lines()
            .any(|line| line.contains("Cloud cover") && line.contains("100%")));
        assert!(find_in_buffer(&buffer, "×0.50").is_none());
    }
}

#[test]
fn test_phase8_weather_policy_explainability_no_fabricated_metrics() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    // Dark phase stays concise: weather does not repeat policy internals.
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.solar_elevation = Some(-15.0); // Night
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(75),
            Some(0.60),
            8,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Night floor").is_none());
        assert!(find_in_buffer(&buffer, "Dark phase").is_none());
    }

    // Daylight targets remain owned by Automation rather than masquerading as
    // hardware telemetry on the Weather screen.
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.solar_elevation = Some(35.0); // Daylight
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(40),
            Some(0.85),
            8,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Solar target").is_none());
        assert!(find_in_buffer(&buffer, "Weather target").is_none());
    }
}

#[test]
fn test_phase8_weather_forecast_integrity_empty_single_and_multiple() {
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    // Empty forecast: no panic
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(30),
            Some(0.9),
            0,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "No forecast samples yet.").is_some());
    }

    // 1 forecast sample: no panic on chart axes
    {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(30),
            Some(0.9),
            1,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
    }
}

#[test]
fn test_phase8_async_shimmer_and_motion_lifecycle() {
    let mut model = Model::new();
    let now = std::time::Instant::now();

    // 1. Opening Tab 4 alone does NOT trigger fake shimmer
    model.active_tab = Tab::Weather;
    assert_eq!(model.motion.active_activity, None);
    assert_eq!(
        model
            .motion
            .activity_shimmer_phase(now, std::time::Duration::from_millis(1500)),
        None
    );

    // 2. Real async activity started under MotionLevel::Instrument
    model.motion.level = crate::config::MotionLevel::Instrument;
    model
        .motion
        .start_activity(super::motion::ActiveActivity::WeatherRefresh { started_at: now });
    assert!(model.motion.active_activity.is_some());

    let phase = model.motion.activity_shimmer_phase(
        now + std::time::Duration::from_millis(750),
        std::time::Duration::from_millis(1500),
    );
    assert!(phase.is_some());
    let p = phase.unwrap();
    assert!((p - 0.5).abs() < 0.05);

    // 3. MotionLevel::Reduced suppresses shimmer animation
    model.motion.level = crate::config::MotionLevel::Reduced;
    assert_eq!(
        model
            .motion
            .activity_shimmer_phase(now, std::time::Duration::from_millis(1500)),
        None
    );

    // 4. MotionLevel::Off suppresses shimmer animation
    model.motion.level = crate::config::MotionLevel::Off;
    assert_eq!(
        model
            .motion
            .activity_shimmer_phase(now, std::time::Duration::from_millis(1500)),
        None
    );

    // 5. Activity stopped explicitly
    model.motion.stop_activity();
    assert_eq!(model.motion.active_activity, None);

    // 6. Geometric invariance: shimmer_style_for_column preserves cell layout
    let styles = super::theme::SemanticStyles::from_palette(&crate::config::Theme::Amber.palette());
    let style_col0 = super::motion::shimmer_style_for_column(
        0,
        20,
        std::time::Duration::from_millis(100),
        std::time::Duration::from_millis(1500),
        &styles,
    );
    assert!(style_col0.fg.is_some());

    // 7. Successful refresh settle confirmation (WeatherUpdate)
    model.motion.level = crate::config::MotionLevel::Instrument;
    model.motion.trigger(
        super::motion::TransientKind::WeatherUpdate,
        now,
        std::time::Duration::from_millis(650),
    );
    assert!(model.motion.weather_update_phase(now).is_some());
    assert!(model
        .motion
        .weather_update_phase(now + std::time::Duration::from_millis(700))
        .is_none());
}

#[test]
fn test_phase8_weather_responsive_comfortable_compact_minimal() {
    let mut model = Model::new();
    model.config.weather.enabled = true;
    model.active_tab = Tab::Weather;
    model.daemon_connection = super::DaemonConnection::Connected;
    let mut status = dummy_status(1);
    status.weather = Some(dummy_weather_status(
        true,
        true,
        false,
        Some(55),
        Some(0.8),
        8,
    ));
    model.status = Some(status);

    // 1. Comfortable keeps exact forecast samples and a real graph.
    {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
        assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
    }

    // 2. Compact (65x15)
    {
        let backend = TestBackend::new(65, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
    }

    // 3. Minimal (50x12)
    {
        let backend = TestBackend::new(50, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
    }
}

#[test]
fn test_phase8_weather_themes_visual_hierarchy() {
    for theme in [
        crate::config::Theme::Amber,
        crate::config::Theme::Nord,
        crate::config::Theme::HackerGreen,
        crate::config::Theme::Grayscale,
        crate::config::Theme::Commodore64,
    ] {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.theme = theme;
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = super::DaemonConnection::Connected;
        let mut status = dummy_status(1);
        status.weather = Some(dummy_weather_status(
            true,
            true,
            false,
            Some(35),
            Some(0.9),
            8,
        ));
        model.status = Some(status);

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
        assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Phase 8.1: Weather Semantic Correctness & Provenance Lock Tests
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_phase8_1_first_snapshot_sync_and_subsequent_updates() {
    let mut model = Model::new();
    model.motion.level = crate::config::MotionLevel::Instrument;
    let now = std::time::Instant::now();

    // 1. Initial state: no baseline established yet
    assert_eq!(model.last_weather_fetched_at, None);
    assert_eq!(model.motion.weather_update_phase(now), None);

    // 2. First observed status packet: establishes baseline, NO confirmation transient
    let mut status1 = dummy_status(1);
    let mut ws1 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
    ws1.fetched_at_epoch_s = Some(1_700_000_000);
    status1.weather = Some(ws1);
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::Status(Box::new(status1))),
    );

    assert_eq!(model.last_weather_fetched_at, Some(1_700_000_000));
    assert_eq!(model.motion.weather_update_phase(now), None);

    // 3. Same timestamp: synchronization / periodic status, NO confirmation transient
    let mut status2 = dummy_status(1);
    let mut ws2 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
    ws2.fetched_at_epoch_s = Some(1_700_000_000);
    status2.weather = Some(ws2);
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::Status(Box::new(status2))),
    );

    assert_eq!(model.last_weather_fetched_at, Some(1_700_000_000));
    assert_eq!(model.motion.weather_update_phase(now), None);

    // 4. Older timestamp: out-of-order delivery, NO confirmation transient
    let mut status3 = dummy_status(1);
    let mut ws3 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
    ws3.fetched_at_epoch_s = Some(1_699_999_999);
    status3.weather = Some(ws3);
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::Status(Box::new(status3))),
    );

    assert_eq!(model.last_weather_fetched_at, Some(1_700_000_000));
    assert_eq!(model.motion.weather_update_phase(now), None);

    // 5. Strictly newer timestamp: genuine fetch completion, TRIGGERS confirmation transient
    let mut status4 = dummy_status(1);
    let mut ws4 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
    ws4.fetched_at_epoch_s = Some(1_700_001_800);
    status4.weather = Some(ws4);
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::Status(Box::new(status4))),
    );

    assert_eq!(model.last_weather_fetched_at, Some(1_700_001_800));
    assert!(model
        .motion
        .weather_update_phase(std::time::Instant::now())
        .is_some());

    // 6. MotionLevel::Off: strictly newer timestamp updates baseline but suppresses transient
    model.motion.active_transient = None;
    model.motion.level = crate::config::MotionLevel::Off;
    let mut status5 = dummy_status(1);
    let mut ws5 = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
    ws5.fetched_at_epoch_s = Some(1_700_003_600);
    status5.weather = Some(ws5);
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::Status(Box::new(status5))),
    );

    assert_eq!(model.last_weather_fetched_at, Some(1_700_003_600));
    assert_eq!(
        model.motion.weather_update_phase(std::time::Instant::now()),
        None
    );
}

#[test]
fn test_phase8_1_freshness_boundaries_refresh_interval_and_cache_ttl() {
    let mut model = Model::new();
    model.config.weather.enabled = true;
    model.config.weather.refresh_minutes = 30; // 30m refresh interval -> 1800s, 60m TTL -> 3600s
    let base_fetch = 1_700_000_000u64;

    // A. age = refresh_interval - 1 (1799s) -> Fresh
    {
        let mut status = dummy_status(1);
        status.now_epoch_s = base_fetch + 1799;
        let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws.fetched_at_epoch_s = Some(base_fetch);
        status.weather = Some(ws);
        model.status = Some(status);

        let telem = super::ui::weather_model::atmospheric_telemetry(&model);
        assert_eq!(
            telem.freshness,
            super::ui::weather_model::FreshnessState::Fresh
        );
        assert_eq!(telem.freshness_label, "● Fresh");
    }

    // B. age = refresh_interval (1800s) -> Fresh (boundary inclusive)
    {
        let mut status = dummy_status(1);
        status.now_epoch_s = base_fetch + 1800;
        let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws.fetched_at_epoch_s = Some(base_fetch);
        status.weather = Some(ws);
        model.status = Some(status);

        let telem = super::ui::weather_model::atmospheric_telemetry(&model);
        assert_eq!(
            telem.freshness,
            super::ui::weather_model::FreshnessState::Fresh
        );
        assert_eq!(telem.freshness_label, "● Fresh");
    }

    // C. age = refresh_interval + 1 (1801s) -> RefreshDue (within cache TTL)
    {
        let mut status = dummy_status(1);
        status.now_epoch_s = base_fetch + 1801;
        let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws.fetched_at_epoch_s = Some(base_fetch);
        status.weather = Some(ws);
        model.status = Some(status);

        let telem = super::ui::weather_model::atmospheric_telemetry(&model);
        assert_eq!(
            telem.freshness,
            super::ui::weather_model::FreshnessState::RefreshDue
        );
        assert_eq!(telem.freshness_label, "○ Refresh Due");
    }

    // D. age = cache_ttl - 1 (3599s) -> RefreshDue (cache remains valid)
    {
        let mut status = dummy_status(1);
        status.now_epoch_s = base_fetch + 3599;
        let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws.fetched_at_epoch_s = Some(base_fetch);
        status.weather = Some(ws);
        model.status = Some(status);

        let telem = super::ui::weather_model::atmospheric_telemetry(&model);
        assert_eq!(
            telem.freshness,
            super::ui::weather_model::FreshnessState::RefreshDue
        );
    }

    // E. age = cache_ttl (3600s) -> RefreshDue (boundary inclusive in core cache_is_fresh)
    {
        let mut status = dummy_status(1);
        status.now_epoch_s = base_fetch + 3600;
        let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
        ws.fetched_at_epoch_s = Some(base_fetch);
        status.weather = Some(ws);
        model.status = Some(status);

        let telem = super::ui::weather_model::atmospheric_telemetry(&model);
        assert_eq!(
            telem.freshness,
            super::ui::weather_model::FreshnessState::RefreshDue
        );
    }

    // F. age = cache_ttl + 1 (3601s) -> Stale (exceeds TTL, excluded from automation)
    {
        let mut status = dummy_status(1);
        status.now_epoch_s = base_fetch + 3601;
        let mut ws = dummy_weather_status(true, false, true, Some(50), None, 8);
        ws.fetched_at_epoch_s = Some(base_fetch);
        status.weather = Some(ws);
        model.status = Some(status);

        let telem = super::ui::weather_model::atmospheric_telemetry(&model);
        assert_eq!(
            telem.freshness,
            super::ui::weather_model::FreshnessState::Stale
        );
        assert_eq!(telem.freshness_label, "▲ Stale");
    }
}

#[test]
fn test_phase8_1_policy_provenance_and_stale_behavior() {
    let mut model = Model::new();
    model.config.weather.enabled = true;
    model.active_tab = Tab::Weather;
    model.daemon_connection = super::DaemonConnection::Connected;

    // 1. Stale cache: multiplier is None, excluded from automation, solar curve unattenuated
    {
        let mut status = dummy_status(1);
        status.solar_elevation = Some(45.0); // Daylight
        status.now_epoch_s = 1_700_005_000;
        let mut ws = dummy_weather_status(true, false, true, Some(60), None, 8);
        ws.fetched_at_epoch_s = Some(1_700_000_000);
        status.weather = Some(ws);
        model.status = Some(status);

        let policy = super::ui::weather_model::compute_atmospheric_policy(&model);
        assert_eq!(policy.multiplier, None);
        assert_eq!(policy.delta_percent, Some(0));
        assert!(policy
            .explanation
            .contains("Stale weather data excluded from automation"));
        assert_eq!(policy.solar_target_percent, policy.effective_target_percent);
    }

    // 2. Failed refresh with valid cache (within TTL): retains multiplier & applies attenuation
    {
        let mut status = dummy_status(1);
        status.solar_elevation = Some(45.0);
        status.now_epoch_s = 1_700_002_000; // 33m old, within 60m TTL
        let mut ws = dummy_weather_status(true, true, false, Some(60), Some(0.75), 8);
        ws.last_error = Some(String::from("openweather timeout"));
        ws.fetched_at_epoch_s = Some(1_700_000_000);
        status.weather = Some(ws);
        model.status = Some(status);

        let telem = super::ui::weather_model::atmospheric_telemetry(&model);
        assert_eq!(
            telem.freshness,
            super::ui::weather_model::FreshnessState::RefreshFailedWithCache
        );
        assert_eq!(telem.policy.multiplier, Some(0.75));
        assert!(telem.policy.delta_percent.is_some());
    }

    // 3. Disabled weather: multiplier is None, explanation notes disabled state
    {
        model.config.weather.enabled = false;
        let policy = super::ui::weather_model::compute_atmospheric_policy(&model);
        assert_eq!(policy.multiplier, None);
        assert!(policy.explanation.contains("Weather disabled"));
    }
}

#[test]
fn test_phase8_1_forecast_provenance_and_timezone_sampling() {
    // 1. Timezone and time-format correctness
    // 1700000000 = 2023-11-14 22:13:20 UTC
    // In Europe/Istanbul (UTC+3), this is 2023-11-15 01:13:20
    let label_24h =
        super::ui::weather_model::forecast_time_label(1_700_000_000, false, "Europe/Istanbul");
    assert_eq!(label_24h, "01:13");

    let label_12h =
        super::ui::weather_model::forecast_time_label(1_700_000_000, true, "Europe/Istanbul");
    assert_eq!(label_12h, "01:13 AM");

    // In America/New_York (UTC-5 in Nov, EST), 22:13 UTC is 17:13 (5:13 PM)
    let label_ny =
        super::ui::weather_model::forecast_time_label(1_700_000_000, false, "America/New_York");
    assert_eq!(label_ny, "17:13");

    // 2. Downsampling behavior: in narrow viewport (< 55 cols for 8 points), table downsamples by step=2
    let mut model = Model::new();
    model.config.weather.enabled = true;
    model.active_tab = Tab::Weather;
    model.daemon_connection = super::DaemonConnection::Connected;
    let mut status = dummy_status(1);
    status.weather = Some(dummy_weather_status(
        true,
        true,
        false,
        Some(40),
        Some(0.85),
        8,
    ));
    model.status = Some(status);

    // Narrow terminal (52x20) remains a compact weather composition.
    let backend = TestBackend::new(52, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Renders cleanly without panic or clipping in compact mode
    assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
}

#[test]
fn test_phase8_1_no_fake_shimmer_and_no_fictional_condition_labels() {
    let mut model = Model::new();
    model.config.weather.enabled = true;
    model.active_tab = Tab::Weather;
    model.daemon_connection = super::DaemonConnection::Connected;
    let mut status = dummy_status(1);
    status.weather = Some(dummy_weather_status(
        true,
        true,
        false,
        Some(45),
        Some(0.8),
        8,
    ));
    model.status = Some(status);

    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // 1. No fake shimmer started merely by opening or rendering Tab 4
    assert_eq!(model.motion.active_activity, None);

    // 2. The normalized semantic condition is shown once, with no invented
    // alternate condition labels.
    assert!(find_in_buffer(&buffer, "Partly cloudy").is_some());
    assert!(find_in_buffer(&buffer, "Partly Cloudy").is_none());
}

// ══════════════════════════════════════════════════════════════════════════
// Phase 8.2: Weather Temporal Truth + Atmospheric Configuration Consolidation
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_phase8_2_atmospheric_input_valid_time_and_temporal_truth() {
    let mut model = Model::new();
    model.config.weather.enabled = true;
    model.config.location.timezone = String::from("Europe/Istanbul");
    model.config.tui.use_12h_time = false;
    model.active_tab = Tab::Weather;
    model.daemon_connection = super::DaemonConnection::Connected;

    let mut status = dummy_status(1);
    let mut ws = dummy_weather_status(true, true, false, Some(55), Some(0.8), 8);
    // 1700000000 UTC is 2023-11-14 22:13:20 UTC -> Istanbul UTC+3 = 01:13:20
    ws.valid_at_epoch_s = Some(1_700_000_000);
    status.weather = Some(ws);
    model.status = Some(status);

    // 1. The screen uses the normalized forecast, not a fake "current sky"
    // label. Time conversion is covered by the pure helper above.
    {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "CURRENT SKY").is_none());
        assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
    }

    // 2. The 12-hour preference continues to render without a layout failure.
    {
        model.config.tui.use_12h_time = true;
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
    }

    // 3. A different IANA timezone also leaves the normalized rendering intact.
    {
        model.config.location.timezone = String::from("America/New_York");
        model.config.tui.use_12h_time = false;
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "24 hour forecast").is_some());
    }
}

#[test]
fn test_phase8_2_refresh_due_never_claims_in_flight() {
    let mut model = Model::new();
    model.config.weather.enabled = true;
    model.active_tab = Tab::Weather;
    model.daemon_connection = super::DaemonConnection::Connected;

    let base_fetch = 1_700_000_000;
    let mut status = dummy_status(1);
    status.now_epoch_s = base_fetch + 2820; // 47m after fetch (refresh due at 30m, TTL at 60m)
    let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.8), 8);
    ws.fetched_at_epoch_s = Some(base_fetch);
    status.weather = Some(ws);
    model.status = Some(status);

    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // A stale refresh schedule is not presented as an in-flight operation.
    assert!(find_in_buffer(&buffer, "Refresh due").is_some());
    assert!(find_in_buffer(&buffer, "updated 47 min ago").is_some());

    // Must NEVER claim in-flight, fetching, or downloading
    assert!(find_in_buffer(&buffer, "in flight").is_none());
    assert!(find_in_buffer(&buffer, "refresh scheduled").is_none());
    assert!(find_in_buffer(&buffer, "fetching").is_none());
    assert!(find_in_buffer(&buffer, "downloading").is_none());
}

#[test]
fn test_phase8_2_policy_target_never_masquerades_as_applied_hardware() {
    let mut model = Model::new();
    model.config.weather.enabled = true;
    model.active_tab = Tab::Weather;
    model.daemon_connection = super::DaemonConnection::Connected;

    let mut status = dummy_status(1);
    status.solar_elevation = Some(40.0);
    // Monitor hardware applied is 55%, with an active manual override of 80%
    if let Some(mon) = status.monitors.get_mut(0) {
        mon.last_applied_percent = Some(55);
        mon.override_percent = Some(80);
    }
    let mut ws = dummy_weather_status(true, true, false, Some(50), Some(0.85), 8);
    ws.valid_at_epoch_s = Some(1_700_000_000);
    status.weather = Some(ws);
    model.status = Some(status);

    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Weather does not present Automation policy targets as live hardware
    // telemetry; its role is current conditions and forecast.
    assert!(find_in_buffer(&buffer, "Solar target").is_none());
    assert!(find_in_buffer(&buffer, "Weather target").is_none());

    // Weather must NOT claim policy target is applied or confirmed hardware
    assert!(find_in_buffer(&buffer, "Applied").is_none());
    assert!(find_in_buffer(&buffer, "Current hardware").is_none());
    assert!(find_in_buffer(&buffer, "Confirmed brightness").is_none());
}

#[test]
fn test_phase8_2_settings_atmosphere_consolidation_and_editing() {
    let mut model = Model::new();
    model.active_tab = Tab::Settings;
    model.active_setting = super::model::settings_index::WEATHER_ENABLED;
    model.daemon_connection = super::DaemonConnection::Connected;
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
        model.active_setting = super::model::settings_index::WEATHER_ENABLED;
        let initial_enabled = model.config.weather.enabled;

        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Enter,
            )),
        );
        assert_eq!(model.config.weather.enabled, !initial_enabled);
    }

    // 4. API Key editing and cancel/save in Settings
    {
        model.active_setting = super::model::settings_index::WEATHER_API_KEY;
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Enter,
            )),
        );
        assert!(matches!(model.input_mode, super::model::InputMode::Editing));

        // Type new secret
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('k'),
            )),
        );

        // Cancel with Esc
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Esc,
            )),
        );
        assert!(matches!(model.input_mode, super::model::InputMode::Normal));
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Phase 9: Operator Help / Command Reference Overlay & Shared Command Grammar
// ══════════════════════════════════════════════════════════════════════════

#[test]
#[allow(clippy::too_many_lines)]
fn test_phase9_context_aware_help_all_contexts() {
    let mut model = Model::new();
    model.show_help = true;

    let render = |model: &mut Model| {
        let mut terminal = Terminal::new(TestBackend::new(85, 26)).unwrap();
        terminal.draw(|f| ui::ui(f, model)).unwrap();
        terminal.backend().buffer().clone()
    };
    let assert_all = |buffer: &ratatui::buffer::Buffer, texts: &[&str]| {
        for text in texts {
            assert!(
                find_in_buffer(buffer, text).is_some(),
                "{text:?} missing:\n{}",
                buffer_text(buffer)
            );
        }
    };

    // 1. Monitors list
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::List;
    model.input_mode = InputMode::Normal;
    let buffer = render(&mut model);
    assert_all(
        &buffer,
        &[
            "╭ Help",
            "Select monitor",
            "Edit brightness range",
            "Open automation for this monitor",
            "Everywhere",
            "Symbols",
        ],
    );

    // 2. Monitors range controls
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    let buffer = render(&mut model);
    assert_all(
        &buffer,
        &[
            "Choose minimum or maximum",
            "Enter exact brightness value",
            "Adjust selected brightness ±1%",
            "Back to monitor list",
        ],
    );

    // 3. Monitors edit: global shortcuts are not advertised while typing.
    model.input_mode = InputMode::Editing;
    let buffer = render(&mut model);
    assert_all(
        &buffer,
        &["Input percentage value", "Save and apply", "Cancel editing"],
    );
    assert!(find_in_buffer(&buffer, "Everywhere").is_none());

    // 4. Automation curve (the default Automation focus)
    model.active_tab = Tab::Limits;
    model.input_mode = InputMode::Normal;
    model.automation_focus = super::model::AutomationRegionFocus::Curve;
    let buffer = render(&mut model);
    assert_all(
        &buffer,
        &[
            "Solar curvature",
            "Switch monitor",
            "Adjust gamma by 0.05",
            "Move to the schedule",
        ],
    );

    // 5. Automation schedule
    model.automation_focus = super::model::AutomationRegionFocus::Milestones;
    let buffer = render(&mut model);
    assert_all(
        &buffer,
        &[
            "Automation schedule",
            "Shift the selected milestone by 1 minute",
            "Reset the milestone to its solar time",
        ],
    );
    assert!(find_in_buffer(&buffer, "Left/Right change monitor").is_none());

    // 6. Location
    model.active_tab = Tab::Location;
    let buffer = render(&mut model);
    assert_all(&buffer, &["Select location field", "Edit selected field"]);

    // 7. City search
    model.active_setting = 0;
    model.input_mode = InputMode::Editing;
    let buffer = render(&mut model);
    assert_all(
        &buffer,
        &[
            "Search city",
            "Search worldwide cities",
            "Select autocomplete match",
            "Use city, coordinates, and timezone",
        ],
    );

    // 8. Settings
    model.active_tab = Tab::Settings;
    model.input_mode = InputMode::Normal;
    let buffer = render(&mut model);
    assert_all(&buffer, &["Select setting", "Toggle or edit setting"]);

    // 9. Settings edit
    model.active_setting = super::model::settings_index::WEATHER_API_KEY;
    model.input_mode = InputMode::Editing;
    let buffer = render(&mut model);
    assert_all(
        &buffer,
        &[
            "Edit setting",
            "Input replacement value",
            "Save configuration",
        ],
    );

    // 10. Weather without data offers no fake controls.
    model.active_tab = Tab::Weather;
    model.input_mode = InputMode::Normal;
    let buffer = render(&mut model);
    assert_all(&buffer, &["Nothing to adjust here."]);
    assert!(find_in_buffer(&buffer, "Edit operating range").is_none());
    assert!(find_in_buffer(&buffer, "Edit API key").is_none());
    assert!(find_in_buffer(&buffer, "Observational atmospheric").is_none());
}

#[test]
fn test_phase9_modal_keys_open_close_and_q() {
    let mut model = Model::new();
    assert!(!model.show_help);

    // 1. '?' opens help
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(model.show_help);

    // 2. '?' again closes help
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(!model.show_help);

    // 3. '?' opens, Esc closes
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(model.show_help);
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc,
        )),
    );
    assert!(!model.show_help);

    // 4. '?' opens, 'q' closes help without quitting application
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(model.show_help);
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('q'),
        )),
    );
    assert!(!model.show_help);
    assert!(!model.should_quit); // Must NOT quit the app
}

#[test]
fn test_phase9_single_source_of_truth_footer_and_help() {
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
        let ctx_cmd = super::command::commands_for_workspace(&model);

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

#[test]
fn test_phase9_no_stale_or_prohibited_terms() {
    let mut model = Model::new();
    model.show_help = true;
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    for tab in [
        Tab::Monitors,
        Tab::Limits,
        Tab::Location,
        Tab::Weather,
        Tab::Settings,
    ] {
        model.active_tab = tab;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // 1. No user-facing "Limits" string in Help
        assert!(find_in_buffer(&buffer, "Limits").is_none());
        assert!(find_in_buffer(&buffer, "LIMITS").is_none());

        // 2. No "fine-tuning" hidden-page terminology
        assert!(find_in_buffer(&buffer, "fine-tuning").is_none());
        assert!(find_in_buffer(&buffer, "fine-tune").is_none());

        // 3. No old "fine-adjust transition gamma"
        assert!(find_in_buffer(&buffer, "fine-adjust transition").is_none());
    }
}

#[test]
fn test_phase9_api_key_secret_safety_in_help() {
    let mut model = Model::new();
    model.active_tab = Tab::Settings;
    model.active_setting = super::model::settings_index::WEATHER_API_KEY;
    model.input_mode = InputMode::Editing;
    model.show_help = true;

    // Put a secret string in the input
    let secret = "test_super_secret_api_key_123456789";
    for c in secret.chars() {
        model
            .form
            .api_key_input
            .handle(tui_input::InputRequest::InsertChar(c));
    }

    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Secret text MUST NEVER leak into the Help overlay
    assert!(find_in_buffer(&buffer, secret).is_none());
    assert!(find_in_buffer(&buffer, "test_super_secret").is_none());
    assert!(find_in_buffer(&buffer, "123456789").is_none());
}

#[test]
fn test_phase9_symbol_legend_vocabulary() {
    let mut model = Model::new();
    model.show_help = true;
    model.active_tab = Tab::Monitors;

    let backend = TestBackend::new(85, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    for text in [
        "❯",
        "keyboard cursor",
        "›",
        "selected item in an unfocused list",
        "‹ ›",
        "value changes with ← →",
        "●",
        "current milestone",
        "○",
        "inactive or unavailable",
        "▲",
        "needs attention",
        "⎿",
        "detail for the row above",
        "Effects",
    ] {
        assert!(
            find_in_buffer(&buffer, text).is_some(),
            "{text:?} missing:\n{}",
            buffer_text(&buffer)
        );
    }
    // The legacy rail vocabulary is gone.
    assert!(find_in_buffer(&buffer, "retained context").is_none());
}

#[test]
fn test_phase9_responsive_help_modal_layouts() {
    let mut model = Model::new();
    model.show_help = true;
    model.active_tab = Tab::Monitors;

    for (width, height, extra) in [(85, 26, "Symbols"), (70, 20, "Symbols"), (50, 14, "close")] {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(
            find_in_buffer(&buffer, "╭ Help").is_some(),
            "{width}x{height}"
        );
        assert!(find_in_buffer(&buffer, extra).is_some(), "{width}x{height}");
    }
}

#[test]
fn test_phase9_theme_palettes_help_render() {
    let mut model = Model::new();
    model.show_help = true;
    model.active_tab = Tab::Monitors;

    for theme in [
        crate::tui::theme::Theme::Amber,
        crate::tui::theme::Theme::Nord,
        crate::tui::theme::Theme::TokyoNight,
        crate::tui::theme::Theme::HackerGreen,
        crate::tui::theme::Theme::Grayscale,
        crate::tui::theme::Theme::Commodore64,
    ] {
        model.config.tui.theme = theme;
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert!(find_in_buffer(&buffer, "╭ Help").is_some());
        assert!(find_in_buffer(&buffer, "Select monitor").is_some());
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Phase 9.1: Command Truth, Modal Precedence & Legacy-State Cleanup
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_phase9_1_input_precedence_editing_mode_absorbs_question_mark() {
    let mut model = Model::new();
    model.active_tab = Tab::Location;
    model.active_setting = 3; // Timezone field (text)
    model.form.timezone_input = tui_input::Input::default().with_value(String::from("UTC"));

    // Enter editing mode
    model.start_editing();
    assert_eq!(model.input_mode, InputMode::Editing);
    assert!(!model.show_help);

    // Typing '?' into text input must be absorbed into buffer, NOT open help
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(!model.show_help);
    assert_eq!(model.input_mode, InputMode::Editing);
    assert_eq!(model.form.timezone_input.value(), "UTC?");

    // Numeric input rejects '?'
    model.stop_editing();
    model.active_tab = Tab::Settings;
    model.active_setting = super::model::settings_index::REFRESH_RATE;
    model.form.fps_input = tui_input::Input::default().with_value(String::from("60"));
    model.start_editing();

    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(!model.show_help);
    assert_eq!(model.input_mode, InputMode::Editing);
    assert_eq!(model.form.fps_input.value(), "60");

    // In edit mode, footer reflects edit commands with no global commands
    let footer_cmds = super::command::commands_for_footer(&model);
    assert!(footer_cmds.global_commands.is_empty());
}

#[test]
fn test_phase9_1_modal_precedence_q_esc_question_mark_state_preservation() {
    let mut model = Model::new();
    model.status = Some(dummy_status(2));
    model.active_tab = Tab::Monitors;
    model.selected_monitor = 1;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    model.monitor_control_index = 1;

    let snapshot_before = (
        model.active_tab,
        model.selected_monitor,
        model.monitor_pane_focus,
        model.monitor_control_index,
        model.should_quit,
    );

    // 1. Open Help via '?'
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(model.show_help);

    // 2. Close Help via 'q'
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('q'),
        )),
    );
    assert!(!model.show_help);
    let snapshot_after_q = (
        model.active_tab,
        model.selected_monitor,
        model.monitor_pane_focus,
        model.monitor_control_index,
        model.should_quit,
    );
    assert_eq!(snapshot_before, snapshot_after_q);

    // 3. Open Help via '?', close via Esc
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(model.show_help);
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc,
        )),
    );
    assert!(!model.show_help);
    let snapshot_after_esc = (
        model.active_tab,
        model.selected_monitor,
        model.monitor_pane_focus,
        model.monitor_control_index,
        model.should_quit,
    );
    assert_eq!(snapshot_before, snapshot_after_esc);

    // 4. Open Help via '?', close via '?'
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(model.show_help);
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('?'),
        )),
    );
    assert!(!model.show_help);
    let snapshot_after_qm = (
        model.active_tab,
        model.selected_monitor,
        model.monitor_pane_focus,
        model.monitor_control_index,
        model.should_quit,
    );
    assert_eq!(snapshot_before, snapshot_after_qm);
}

#[test]
fn test_phase9_1_a_destination_shortcut_semantics() {
    let mut model = Model::new();
    model.status = Some(dummy_status(2));
    model.active_tab = Tab::Monitors;
    model.selected_monitor = 1;

    // 'a' from Monitors switches to Automation (Tab::Limits)
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('a'),
        )),
    );
    assert_eq!(model.active_tab, Tab::Limits);
    assert_eq!(model.selected_monitor, 1); // Selected monitor preserved

    // 'a' from Automation remains in Automation (no-op)
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('a'),
        )),
    );
    assert_eq!(model.active_tab, Tab::Limits); // Still Automation!
    assert_eq!(model.selected_monitor, 1);

    // '1' returns to Monitors
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('1'),
        )),
    );
    assert_eq!(model.active_tab, Tab::Monitors);
    assert_eq!(model.selected_monitor, 1);

    // '2' goes to Automation
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('2'),
        )),
    );
    assert_eq!(model.active_tab, Tab::Limits);
    assert_eq!(model.selected_monitor, 1);
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_phase9_1_command_spec_conformance_dispatch() {
    let mut model = Model::new();
    model.status = Some(dummy_status(2));
    model.config.monitors = vec![
        crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            transition_gamma: 0.5,
            ..Default::default()
        },
        crate::config::MonitorConfig {
            logical_id: String::from("mon-1"),
            transition_gamma: 0.5,
            ..Default::default()
        },
    ];

    // 1. Monitors List
    {
        model.active_tab = Tab::Monitors;
        model.selected_monitor = 0;
        model.monitor_pane_focus = super::model::MonitorPaneFocus::List;

        // Down changes selection
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Down,
            )),
        );
        assert_eq!(model.selected_monitor, 1);

        // Enter focuses detail
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Enter,
            )),
        );
        assert_eq!(
            model.monitor_pane_focus,
            super::model::MonitorPaneFocus::Detail
        );
    }

    // 2. Monitors Detail
    {
        // Esc returns to list
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Esc,
            )),
        );
        assert_eq!(
            model.monitor_pane_focus,
            super::model::MonitorPaneFocus::List
        );

        // Enter again to detail
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Enter,
            )),
        );
        assert_eq!(
            model.monitor_pane_focus,
            super::model::MonitorPaneFocus::Detail
        );

        // Enter from detail starts editing
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Enter,
            )),
        );
        assert_eq!(model.input_mode, InputMode::Editing);
        model.stop_editing();

        // Curve shape has no hidden mutation path from Monitors.
        let initial_gamma = model.config.monitors[1].transition_gamma;
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('+'),
            )),
        );
        let gamma_after = model.config.monitors[1].transition_gamma;
        assert!((gamma_after - initial_gamma).abs() < 1e-4);
    }

    // 3. Automation
    {
        model.active_tab = Tab::Limits;
        model.selected_monitor = 0;
        model.selected_monitor_id = None;
        model.clamp_monitor_selection();
        model.selected_monitor_milestone = 0;
        model.automation_focus = super::model::AutomationRegionFocus::Curve;

        let initial_curve = model.config.monitors[0].transition_gamma;
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Right,
            )),
        );
        assert!((model.config.monitors[0].transition_gamma - (initial_curve + 0.05)).abs() < 1e-4);

        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Down,
            )),
        );
        assert_eq!(
            model.automation_focus,
            super::model::AutomationRegionFocus::Milestones
        );

        let now = model.current_local_time();
        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![
                MonitorMilestone {
                    milestone: AutomationMilestone::RiseStart,
                    base_time_local: now,
                    adjusted_time_local: now,
                    target_percent: 10,
                    minutes_offset: 0,
                },
                MonitorMilestone {
                    milestone: AutomationMilestone::Rise25,
                    base_time_local: now,
                    adjusted_time_local: now,
                    target_percent: 30,
                    minutes_offset: 0,
                },
            ],
        }];

        // Down selects next milestone
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Down,
            )),
        );
        assert_eq!(model.selected_monitor_milestone, 1);

        // Right adjusts offset +1m
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Right,
            )),
        );
        let offset = model
            .config
            .monitors
            .first()
            .and_then(|m| {
                m.milestone_adjustments
                    .iter()
                    .find(|adj| adj.milestone == AutomationMilestone::Rise25)
                    .map(|adj| adj.minutes_offset)
            })
            .unwrap_or(0);
        assert_eq!(offset, 1);

        // 'r' resets offset
        super::update::update(
            &mut model,
            super::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('r'),
            )),
        );
        let offset_reset = model
            .config
            .monitors
            .first()
            .and_then(|m| {
                m.milestone_adjustments
                    .iter()
                    .find(|adj| adj.milestone == AutomationMilestone::Rise25)
                    .map(|adj| adj.minutes_offset)
            })
            .unwrap_or(0);
        assert_eq!(offset_reset, 0);
    }

    // 4. Weather context has no editable commands
    {
        model.active_tab = Tab::Weather;
        let weather_cmds = super::command::commands_for_workspace(&model);
        assert!(weather_cmds.current_commands.is_empty());
    }
}

#[test]
fn test_phase9_1_api_key_secret_safety_comprehensive() {
    let mut model = Model::new();
    model.active_tab = Tab::Settings;
    let secret = "very_secret_api_key_never_leak_xyz";
    model.form.api_key_input = tui_input::Input::default().with_value(String::from(secret));

    // 1. Normal mode: TestBackend buffer shows fixed 8-bullet mask, never raw secret or length
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    assert!(find_in_buffer(&buffer, "●●●●●●●●").is_some());
    assert!(find_in_buffer(&buffer, secret).is_none());
    assert!(find_in_buffer(&buffer, "very_secret").is_none());

    // 2. Help mode: Help never receives or displays the secret
    model.show_help = true;
    let backend2 = TestBackend::new(85, 26);
    let mut terminal2 = Terminal::new(backend2).unwrap();
    terminal2.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer2 = terminal2.backend().buffer().clone();

    assert!(find_in_buffer(&buffer2, secret).is_none());
    assert!(find_in_buffer(&buffer2, "very_secret").is_none());
}

#[test]
fn test_phase9_1_help_modal_scrolling_boundaries_and_resize() {
    let mut model = Model::new();
    model.show_help = true;

    // Top clamp
    model.scroll_help_up(100);
    assert_eq!(model.help_scroll, 0);

    // Page down (6 lines)
    model.scroll_help_down(6, 20);
    assert_eq!(model.help_scroll, 6);

    // Page up (6 lines)
    model.scroll_help_up(6);
    assert_eq!(model.help_scroll, 0);

    // Bottom clamp
    model.scroll_help_down(50, 15);
    assert_eq!(model.help_scroll, 15);

    // Home
    model.scroll_help_home();
    assert_eq!(model.help_scroll, 0);

    // End
    model.scroll_help_end(18);
    assert_eq!(model.help_scroll, 18);

    // Render with large scroll clamped to smaller viewport
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();
    let res = terminal.draw(|f| ui::ui(f, &mut model));
    assert!(res.is_ok());
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_phase9_1_footer_help_consistency_invariant() {
    use super::command::UiCommandContext;

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
        let cmds = match ctx {
            UiCommandContext::MonitorsList => super::command::commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Monitors;
                m.monitor_pane_focus = super::model::MonitorPaneFocus::List;
                m
            }),
            UiCommandContext::MonitorsDetail => super::command::commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Monitors;
                m.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
                m
            }),
            UiCommandContext::Automation => super::command::commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Limits;
                m
            }),
            UiCommandContext::AutomationCurve => super::command::commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Limits;
                m.automation_focus = super::model::AutomationRegionFocus::Curve;
                m
            }),
            UiCommandContext::LocationNav => super::command::commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Location;
                m
            }),
            UiCommandContext::WeatherObservational => super::command::commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Weather;
                m
            }),
            UiCommandContext::SettingsNav => super::command::commands_for_workspace(&{
                let mut m = Model::new();
                m.active_tab = Tab::Settings;
                m
            }),
            _ => super::command::commands_for_footer(&{
                let mut m = Model::new();
                m.show_help = matches!(ctx, UiCommandContext::HelpModal);
                m
            }),
        };

        // Command specs must be valid and adhere to safety standards
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

            // Footer label must be concise for compact viewports
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

#[test]
fn test_phase9_1_responsive_help_all_contexts_no_panic() {
    let mut model = Model::new();
    model.show_help = true;

    for tab in [
        Tab::Monitors,
        Tab::Limits,
        Tab::Location,
        Tab::Weather,
        Tab::Settings,
    ] {
        model.active_tab = tab;
        for (w, h) in [(85, 26), (70, 20), (50, 14), (30, 10)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok(), "Failed to render Help on {tab:?} at {w}x{h}");
        }
    }
}

#[test]
fn test_phase9_2_terminal_cell_metrics_detection_and_fallback() {
    use super::geometry::{CellAspectSource, TerminalCellMetrics, DEFAULT_CELL_ASPECT};

    // 1. Valid dimensions (80 cols, 24 rows, 640x384 pixels -> 8x16 px cell -> 0.5 aspect)
    let m1 = TerminalCellMetrics::from_raw_dimensions(80, 24, 640, 384);
    assert_eq!(m1.source, CellAspectSource::TerminalPixels);
    assert!((m1.cell_aspect - 0.5).abs() < 1e-6);

    // 2. Alternative valid font (80 cols, 25 rows, 720x375 pixels -> 9x15 px cell -> 0.6 aspect)
    let m2 = TerminalCellMetrics::from_raw_dimensions(80, 25, 720, 375);
    assert_eq!(m2.source, CellAspectSource::TerminalPixels);
    assert!((m2.cell_aspect - 0.6).abs() < 1e-6);

    // 3. Zero columns -> Fallback
    let m3 = TerminalCellMetrics::from_raw_dimensions(0, 24, 640, 384);
    assert_eq!(m3.source, CellAspectSource::Fallback);
    assert!((m3.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

    // 4. Zero rows -> Fallback
    let m4 = TerminalCellMetrics::from_raw_dimensions(80, 0, 640, 384);
    assert_eq!(m4.source, CellAspectSource::Fallback);
    assert!((m4.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

    // 5. Zero pixel width -> Fallback
    let m5 = TerminalCellMetrics::from_raw_dimensions(80, 24, 0, 384);
    assert_eq!(m5.source, CellAspectSource::Fallback);
    assert!((m5.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

    // 6. Zero pixel height -> Fallback
    let m6 = TerminalCellMetrics::from_raw_dimensions(80, 24, 640, 0);
    assert_eq!(m6.source, CellAspectSource::Fallback);
    assert!((m6.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

    // 7. Non-physical aspect < 0.25 -> Fallback
    let m7 = TerminalCellMetrics::from_raw_dimensions(80, 24, 100, 1000);
    assert_eq!(m7.source, CellAspectSource::Fallback);

    // 8. Non-physical aspect > 1.5 -> Fallback
    let m8 = TerminalCellMetrics::from_raw_dimensions(80, 24, 2000, 100);
    assert_eq!(m8.source, CellAspectSource::Fallback);

    // 9. OS detection returns a sane metric
    let detected = super::geometry::detect_terminal_cell_metrics();
    assert!(detected.cell_aspect.is_finite());
    assert!(detected.cell_aspect > 0.0);
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_phase9_2_pure_viewport_geometry_fit_matrix() {
    use super::geometry::{fit_world_map_viewport, WORLD_ASPECT};
    use ratatui::layout::Rect;

    // 1. Square-ish allocated region (50x50, cell aspect 0.5 -> desired cols/row = 4.0)
    // 50 / 50 = 1.0 < 4.0 -> Taller than desired -> constrain width (50), height = round(50/4) = 13.
    {
        let avail = Rect::new(10, 10, 50, 50);
        let fitted = fit_world_map_viewport(avail, 0.5);
        assert_eq!(fitted.width, 50);
        assert_eq!(fitted.height, 13);
        assert_eq!(fitted.x, 10);
        assert_eq!(fitted.y, 10 + (50 - 13) / 2); // centered vertically: 10 + 18 = 28
        let effective_aspect = (f64::from(fitted.width) / f64::from(fitted.height)) * 0.5;
        assert!((effective_aspect - WORLD_ASPECT).abs() < 0.1);
    }

    // 2. Very wide region (300x10, cell aspect 0.5 -> desired cols/row = 4.0)
    // 300 / 10 = 30.0 > 4.0 -> Wider than desired -> constrain height (10), width = round(10*4) = 40.
    {
        let avail = Rect::new(0, 0, 300, 10);
        let fitted = fit_world_map_viewport(avail, 0.5);
        assert_eq!(fitted.width, 40);
        assert_eq!(fitted.height, 10);
        assert_eq!(fitted.x, (300 - 40) / 2); // 130
        assert_eq!(fitted.y, 0);
        let effective_aspect = (f64::from(fitted.width) / f64::from(fitted.height)) * 0.5;
        assert!((effective_aspect - WORLD_ASPECT).abs() < 1e-6);
    }

    // 3. Very tall region (40x60, cell aspect 0.5 -> desired cols/row = 4.0)
    // 40 / 60 = 0.667 < 4.0 -> Taller than desired -> constrain width (40), height = round(40/4) = 10.
    {
        let avail = Rect::new(5, 5, 40, 60);
        let fitted = fit_world_map_viewport(avail, 0.5);
        assert_eq!(fitted.width, 40);
        assert_eq!(fitted.height, 10);
        assert_eq!(fitted.x, 5);
        assert_eq!(fitted.y, 5 + (60 - 10) / 2); // 5 + 25 = 30
        let effective_aspect = (f64::from(fitted.width) / f64::from(fitted.height)) * 0.5;
        assert!((effective_aspect - WORLD_ASPECT).abs() < 1e-6);
    }

    // 4. Exact target-ratio region (80x20, cell aspect 0.5 -> desired cols/row = 4.0)
    // 80 / 20 = 4.0 == 4.0 -> Exact match.
    {
        let avail = Rect::new(0, 0, 80, 20);
        let fitted = fit_world_map_viewport(avail, 0.5);
        assert_eq!(fitted.width, 80);
        assert_eq!(fitted.height, 20);
        assert_eq!(fitted.x, 0);
        assert_eq!(fitted.y, 0);
    }

    // 5. Odd-numbered dimensions (77x23, cell aspect 0.5 -> desired cols/row = 4.0)
    // 77 / 23 = 3.348 < 4.0 -> Constrain width (77), height = round(77/4) = 19.
    {
        let avail = Rect::new(2, 3, 77, 23);
        let fitted = fit_world_map_viewport(avail, 0.5);
        assert_eq!(fitted.width, 77);
        assert_eq!(fitted.height, 19);
        assert_eq!(fitted.x, 2);
        assert_eq!(fitted.y, 3 + (23 - 19) / 2); // 3 + 2 = 5
        assert!(fitted.x >= avail.x && fitted.x + fitted.width <= avail.x + avail.width);
        assert!(fitted.y >= avail.y && fitted.y + fitted.height <= avail.y + avail.height);
    }

    // 6. Tiny valid region (1x1)
    {
        let avail = Rect::new(0, 0, 1, 1);
        let fitted = fit_world_map_viewport(avail, 0.5);
        assert_eq!(fitted.width, 1);
        assert_eq!(fitted.height, 1);
        assert_eq!(fitted.x, 0);
        assert_eq!(fitted.y, 0);
    }

    // 7. Alternative cell aspect (0.6 -> desired cols/row = 2.0 / 0.6 = 3.3333)
    // 100 x 30: 100 / 30 = 3.3333 -> exact match
    {
        let avail = Rect::new(0, 0, 100, 30);
        let fitted = fit_world_map_viewport(avail, 0.6);
        assert_eq!(fitted.width, 100);
        assert_eq!(fitted.height, 30);
    }

    // 8. Missing / non-finite cell aspect falls back safely to DEFAULT_CELL_ASPECT
    {
        let avail = Rect::new(0, 0, 80, 30);
        let fitted_nan = fit_world_map_viewport(avail, f64::NAN);
        let fitted_fallback = fit_world_map_viewport(avail, super::geometry::DEFAULT_CELL_ASPECT);
        assert_eq!(fitted_nan, fitted_fallback);
    }
}

#[test]
fn test_phase9_2_geographic_projection_normalized_invariance() {
    use super::geometry::fit_world_map_viewport;
    use ratatui::layout::Rect;

    // Normalization formulas:
    // norm_x = (lon - (-180.0)) / 360.0
    // norm_y = (lat - (-90.0)) / 180.0
    let coords: [(&str, f64, f64, f64, f64); 5] = [
        ("Center", 0.0, 0.0, 0.5, 0.5),
        (
            "New York",
            40.7128,
            -74.0060,
            (-74.0060 + 180.0) / 360.0,
            (40.7128 + 90.0) / 180.0,
        ),
        (
            "Istanbul",
            41.01384,
            28.94966,
            (28.94966 + 180.0) / 360.0,
            (41.01384 + 90.0) / 180.0,
        ),
        (
            "Sydney",
            -33.8688,
            151.2093,
            (151.2093 + 180.0) / 360.0,
            (-33.8688 + 90.0) / 180.0,
        ),
        (
            "Tokyo",
            35.6762,
            139.6503,
            (139.6503 + 180.0) / 360.0,
            (35.6762 + 90.0) / 180.0,
        ),
    ];

    // Verify directional quadrant invariants
    for (name, _lat, _lon, nx, ny) in coords {
        match name {
            "Center" => {
                assert!((nx - 0.5).abs() < 1e-4);
                assert!((ny - 0.5).abs() < 1e-4);
            }
            "New York" => {
                assert!(nx < 0.5, "New York must be west of Prime Meridian");
                assert!(ny > 0.5, "New York must be north of Equator");
            }
            "Istanbul" | "Tokyo" => {
                assert!(nx > 0.5, "{name} must be east of Prime Meridian");
                assert!(ny > 0.5, "{name} must be north of Equator");
            }
            "Sydney" => {
                assert!(nx > 0.5, "Sydney must be east of Prime Meridian");
                assert!(ny < 0.5, "Sydney must be south of Equator");
            }
            _ => {}
        }
    }

    // Verify invariance of normalized projection across wide, normal, and tall viewports
    let viewports = [
        Rect::new(0, 0, 120, 20), // Wide
        Rect::new(0, 0, 85, 26),  // Normal
        Rect::new(0, 0, 50, 40),  // Tall
    ];

    for avail in viewports {
        let fitted = fit_world_map_viewport(avail, 0.5);
        assert!(fitted.width <= avail.width);
        assert!(fitted.height <= avail.height);

        // For each city, compute projected sub-cell coordinate inside fitted viewport
        for (_name, _lat, _lon, nx, ny) in coords {
            let px = f64::from(fitted.x) + nx * f64::from(fitted.width);
            let py = f64::from(fitted.y) + (1.0 - ny) * f64::from(fitted.height);
            assert!(px >= f64::from(fitted.x) && px <= f64::from(fitted.x + fitted.width));
            assert!(py >= f64::from(fitted.y) && py <= f64::from(fitted.y + fitted.height));
        }
    }
}

#[test]
fn test_phase9_2_visual_aspect_regression_across_viewports() {
    let test_viewports = [
        (120, 20), // Wide
        (100, 30), // Comfortable wide
        (80, 24),  // Standard terminal
        (65, 18),  // Compact stacked
        (50, 30),  // Narrow tall
        (60, 45),  // Very tall
    ];

    for (w, h) in test_viewports {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.location.city = String::from("Istanbul, TR");
        model.config.location.latitude = 41.01384;
        model.config.location.longitude = 28.94966;
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.form.refresh_from_config(&model.config);
        model.active_tab = Tab::Location;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok(), "Location failed to render at {w}x{h}");

        let buffer = terminal.backend().buffer().clone();
        assert!(find_in_buffer(&buffer, "Location").is_some());

        // The globe panel appears whenever the layout can give it room.
        let full_logo = w >= 76 && h >= 32;
        let chrome_rows: u16 = if full_logo { 8 } else { 1 } + 2 + 1;
        let body_h = h.saturating_sub(chrome_rows);
        let body_h = if body_h > 12 { body_h - 1 } else { body_h };
        let wide = w.saturating_sub(2) >= 80 && body_h >= 12;
        if wide || body_h.saturating_sub(6) >= 8 {
            assert!(
                find_in_buffer(&buffer, "Earth").is_some(),
                "Earth missing at {w}x{h}"
            );
        }
    }
}

#[test]
fn test_phase9_2_acquisition_and_reticle_sharing_fitted_canvas() {
    let mut model = Model::new();
    model.config.location.city = String::from("Tokyo, JP");
    model.config.location.latitude = 35.6762;
    model.config.location.longitude = 139.6503;
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;
    model.motion.level = crate::tui::motion::MotionLevel::Instrument;

    // Trigger an active location acquisition transient
    model.motion.trigger(
        super::motion::TransientKind::LocationAcquisition {
            lon: 139.6503,
            lat: 35.6762,
        },
        std::time::Instant::now(),
        std::time::Duration::from_millis(500),
    );

    for (w, h) in [(120, 25), (85, 26), (75, 40)] {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok(), "Acquisition ping failed to render at {w}x{h}");
    }
}

#[test]
fn test_phase9_2_resize_stability_and_cell_metrics_update() {
    let mut model = Model::new();
    model.config.location.city = String::from("Sydney, AU");
    model.config.location.latitude = -33.8688;
    model.config.location.longitude = 151.2093;
    model.config.location.timezone = String::from("Australia/Sydney");
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;

    assert!(model.cell_metrics.cell_aspect > 0.0);

    // Simulate resize event
    super::update::update(&mut model, super::update::Message::Resize(140, 45));

    // Assert location data is never mutated by a resize
    assert_eq!(model.config.location.city, "Sydney, AU");
    assert!((model.config.location.latitude - (-33.8688)).abs() < 1e-6);
    assert!((model.config.location.longitude - 151.2093).abs() < 1e-6);
    assert_eq!(model.config.location.timezone, "Australia/Sydney");
    assert!(model.cell_metrics.cell_aspect > 0.0);
}

// ══════════════════════════════════════════════════════════════════════════
// Phase 9.3: Identity Restoration, Interaction Salience & Product-Language Cleanup Tests
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_phase9_3_dual_tone_brand_and_sweep_lifecycle() {
    let mut model = Model::new();
    model.config.tui.effects = crate::config::MotionLevel::Instrument;
    model.motion.level = crate::config::MotionLevel::Instrument;

    let backend = TestBackend::new(85, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Initial settled render with motion: verify two-tone presence
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Compact brand retains its two-tone Sun/Reactor treatment.
    let (_, _, sun_cell) = find_in_buffer(&buffer, "Sun").expect("Sun brand segment");
    let (_, _, reactor_cell) = find_in_buffer(&buffer, "Reactor").expect("Reactor brand segment");
    assert_ne!(sun_cell.fg, reactor_cell.fg);

    // 2. Effects Off: compact brand renders immediately in its settled state.
    model.config.tui.effects = crate::config::MotionLevel::Off;
    model.motion.level = crate::config::MotionLevel::Off;
    let backend_off = TestBackend::new(85, 30);
    let mut terminal_off = Terminal::new(backend_off).unwrap();
    terminal_off.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_off = terminal_off.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_off, "SunReactor").is_some());

    // 3. Compact mode: does not crash and renders clean title
    let backend_compact = TestBackend::new(65, 18);
    let mut terminal_compact = Terminal::new(backend_compact).unwrap();
    assert!(terminal_compact.draw(|f| ui::ui(f, &mut model)).is_ok());

    // 4. SemanticStyles styles verify two distinct colors for logo
    let palette = model.config.tui.theme.palette();
    let styles = palette.styles();
    assert_ne!(styles.chrome_title, styles.chrome_title_secondary);
}

#[test]
fn test_phase9_3_focus_and_selection_states() {
    let mut model = Model::new();
    model.active_tab = Tab::Settings;
    let palette = model.config.tui.theme.palette();

    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Theme focused: cursor and an accent capsule on its value only.
    model.active_setting = 0;
    model.input_mode = InputMode::Normal;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let (tx, ty, _) = find_in_buffer(&buffer, "Theme").unwrap();
    assert_eq!(buffer.get(tx - 2, ty).symbol(), "❯");
    let (ex, ey, _) = find_in_buffer(&buffer, "Effects").unwrap();
    assert_ne!(buffer.get(ex - 2, ey).symbol(), "❯");
    let (_, _, theme_value) = find_in_buffer(&buffer, model.config.tui.theme.name()).unwrap();
    assert_eq!(theme_value.bg, palette.accent);

    // 2. Animation rate focused.
    model.active_setting = super::model::settings_index::REFRESH_RATE;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer2 = terminal.backend().buffer().clone();
    let (rx, ry, _) = find_in_buffer(&buffer2, "Animation rate").unwrap();
    assert_eq!(buffer2.get(rx - 2, ry).symbol(), "❯");
    let fps = format!("{} fps", model.config.tui.fps);
    let (_, _, fps_cell) = find_in_buffer(&buffer2, &fps).unwrap();
    assert_eq!(fps_cell.bg, palette.accent);

    // 3. Editing uses a distinct capsule colour on the same row.
    model.input_mode = InputMode::Editing;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer3 = terminal.backend().buffer().clone();
    let (rx, ry, _) = find_in_buffer(&buffer3, "Animation rate").unwrap();
    assert!((rx..85).any(|x| buffer3.get(x, ry).bg == palette.secondary_accent));

    // 4. Read-only rows never take the cursor and are muted.
    let (px, py, provider) = find_in_buffer(&buffer, "Provider").unwrap();
    assert_ne!(buffer.get(px - 2, py).symbol(), "❯");
    assert_eq!(provider.fg, palette.text_muted);
}

#[test]
fn test_phase9_3_settings_layout_spacing_and_balance() {
    let mut model = Model::new();
    model.active_tab = Tab::Settings;
    model.daemon_connection = super::DaemonConnection::Connected;
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
fn test_phase9_3_copy_regression_and_forbidden_terms() {
    let mut model = Model::new();
    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();

    // Test across all tabs
    for tab in [
        Tab::Monitors,
        Tab::Limits,
        Tab::Location,
        Tab::Weather,
        Tab::Settings,
    ] {
        model.active_tab = tab;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // 1. Prohibited theatrical, redundant, or architectural jargon strings
        assert!(
            find_in_buffer(&buffer, "ATMOSPHERIC SUBSYSTEM").is_none(),
            "Found ATMOSPHERIC SUBSYSTEM on {tab:?}"
        );
        assert!(
            find_in_buffer(&buffer, "ATMOSPHERIC TELEMETRY").is_none(),
            "Found ATMOSPHERIC TELEMETRY on {tab:?}"
        );
        assert!(
            find_in_buffer(&buffer, "LOCATION IDENTITY").is_none(),
            "Found LOCATION IDENTITY on {tab:?}"
        );
        assert!(
            find_in_buffer(&buffer, "DRIVES AUTOMATION").is_none(),
            "Found DRIVES AUTOMATION on {tab:?}"
        );
        assert!(
            find_in_buffer(&buffer, "OPERATOR REFERENCE").is_none(),
            "Found OPERATOR REFERENCE on {tab:?}"
        );
        assert!(
            find_in_buffer(&buffer, "INSTRUMENT LEGEND").is_none(),
            "Found INSTRUMENT LEGEND on {tab:?}"
        );
        assert!(
            find_in_buffer(&buffer, "Dim automatically after idle").is_none(),
            "Found Dim automatically after idle on {tab:?}"
        );
        assert!(
            find_in_buffer(&buffer, "OpenWeather API key").is_none(),
            "Found OpenWeather API key on {tab:?}"
        );
        assert!(
            find_in_buffer(&buffer, "Solar automation is driving monitor brightness").is_none(),
            "Found paragraph slop on {tab:?}"
        );
    }
}

#[test]
fn test_phase9_3_contrast_safety_across_representative_themes() {
    use crate::config::Theme;
    use ratatui::style::Color;

    for theme in [
        Theme::Amber,
        Theme::Terminal,
        Theme::Nord,
        Theme::TokyoNight,
        Theme::HackerGreen,
        Theme::Grayscale,
        Theme::Commodore64,
        Theme::Synthwave84,
    ] {
        let palette = theme.palette();
        let styles = palette.styles();

        // 1. Dual-tone masthead colors are distinct and valid
        assert_ne!(styles.chrome_title, styles.chrome_title_secondary);

        // 2. Focused value capsule uses palette accent and has contrasting text
        assert_eq!(styles.value_capsule_focused.bg, Some(palette.accent));
        assert_eq!(
            styles.value_capsule_editing.bg,
            Some(palette.secondary_accent)
        );
        assert_ne!(styles.value_capsule_editing.bg, Some(palette.warning));

        // 3. Capsule fg is derived from palette bg or fg or black/white
        let fg = styles.value_capsule_focused.fg.unwrap();
        assert!(fg == palette.bg || fg == palette.fg || fg == Color::Black || fg == Color::White);
    }
}

#[test]
fn test_phase9_3_1_cell_aspect_validity_bounds() {
    use crate::tui::geometry::{CellAspectSource, TerminalCellMetrics, DEFAULT_CELL_ASPECT};

    // 1. Genuine terminal font dimensions (e.g. 9px x 20px -> 0.45)
    let m = TerminalCellMetrics::from_raw_dimensions(120, 30, 1080, 600);
    assert_eq!(m.source, CellAspectSource::TerminalPixels);
    assert!((m.cell_aspect - 0.45).abs() < 1e-4);

    // 2. Reject bogus 1:1 character-count reporting (columns=80, rows=24, width=80, height=24)
    let m_bogus = TerminalCellMetrics::from_raw_dimensions(80, 24, 80, 24);
    assert_eq!(m_bogus.source, CellAspectSource::Fallback);
    assert!((m_bogus.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

    // 3. Reject extreme / non-physical aspects
    let m_zero = TerminalCellMetrics::from_raw_dimensions(80, 24, 0, 0);
    assert_eq!(m_zero.source, CellAspectSource::Fallback);

    let m_flat = TerminalCellMetrics::from_raw_dimensions(80, 24, 1600, 240); // 2.0
    assert_eq!(m_flat.source, CellAspectSource::Fallback);
}

#[test]
fn test_phase9_3_1_settings_empty_row_invariants() {
    let mut model = Model::new();
    model.active_tab = Tab::Settings;

    for (width, height) in [(120, 40), (80, 24)] {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buf = terminal.backend().buffer().clone();

        let (_, temp_y, _) = find_in_buffer(&buf, "Temperature").unwrap();
        let (_, power_y, _) = find_in_buffer(&buf, "Power ─").unwrap();
        assert_eq!(
            power_y,
            temp_y + 2,
            "{width}x{height}: groups need one blank row between them"
        );
    }
}

#[test]
fn test_phase9_3_1_focus_salience_cell_style_and_editing_semantics() {
    let mut model = Model::new();
    model.active_tab = Tab::Settings;
    model.active_setting = 0; // Focus on Theme

    let backend = TestBackend::new(85, 26);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buf = terminal.backend().buffer().clone();

    // Find the focused Theme value.
    let (vx, vy, _) = find_in_buffer(&buf, model.config.tui.theme.name()).unwrap();
    let cell_focused = buf.get(vx, vy);
    // Focused cell must have background color set to accent
    let palette = model.config.tui.theme.palette();
    assert_eq!(cell_focused.style().bg, Some(palette.accent));

    // Now check unfocused setting (Effects at active_setting = 0 is unfocused)
    let (ex, ey, _) = find_in_buffer(&buf, model.config.tui.effects.label()).unwrap();
    let cell_unfocused = buf.get(ex, ey);
    // Unfocused cell does NOT have accent background
    assert_ne!(cell_unfocused.style().bg, Some(palette.accent));

    // Now test Editing mode on Refresh rate.
    model.active_setting = super::model::settings_index::REFRESH_RATE;
    model.input_mode = crate::tui::model::InputMode::Editing;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buf_edit = terminal.backend().buffer().clone();

    // Find the row containing "Animation rate"
    let (rx, ry, _) = find_in_buffer(&buf_edit, "Animation rate").unwrap();
    let found_bg = (rx..85)
        .map(|column| buf_edit.get(column, ry).style().bg)
        .find(|background| *background == Some(palette.secondary_accent))
        .flatten();
    // Editing cell uses secondary accent (item_editing), NOT warning!
    assert_eq!(found_bg, Some(palette.secondary_accent));
    assert_ne!(found_bg, Some(palette.warning));
}

#[test]
fn test_phase9_4_adaptive_map_zoom_aspect_invariance() {
    use crate::tui::geometry::MapZoomLevel;

    // 1. Dimension-based selection
    assert_eq!(MapZoomLevel::select(70, 16, None), MapZoomLevel::World);
    assert_eq!(
        MapZoomLevel::select(45, 10, None),
        MapZoomLevel::Continental
    );
    assert_eq!(MapZoomLevel::select(30, 6, None), MapZoomLevel::Regional);

    // 2. Hysteresis holds the current semantic scale through one-cell resize jitter.
    assert_eq!(
        MapZoomLevel::select(51, 11, Some(MapZoomLevel::World)),
        MapZoomLevel::World
    );
    assert_eq!(
        MapZoomLevel::select(49, 11, Some(MapZoomLevel::World)),
        MapZoomLevel::Continental
    );
    assert_eq!(
        MapZoomLevel::select(33, 7, Some(MapZoomLevel::Continental)),
        MapZoomLevel::Continental
    );
    assert_eq!(
        MapZoomLevel::select(31, 7, Some(MapZoomLevel::Continental)),
        MapZoomLevel::Regional
    );

    // 3. Aspect Ratio Invariance (2:1 at ALL levels)
    for zoom in [
        MapZoomLevel::World,
        MapZoomLevel::Continental,
        MapZoomLevel::Regional,
    ] {
        let (x_bounds, y_bounds) = zoom.bounds(29.0, 41.0);
        let span_x = x_bounds[1] - x_bounds[0];
        let span_y = y_bounds[1] - y_bounds[0];
        let ratio = span_x / span_y;
        assert!(
            (ratio - 2.0).abs() < 1e-6,
            "Zoom {zoom:?} violated 2:1 aspect ratio: {ratio}"
        );

        // Clamping to valid world geography
        assert!(x_bounds[0] >= -180.0 && x_bounds[1] <= 180.0);
        assert!(y_bounds[0] >= -90.0 && y_bounds[1] <= 90.0);
    }

    // 4. Locations remain in bounds through dateline and polar clamping.
    let (x_east, y_north) = MapZoomLevel::Regional.bounds(179.0, 85.0);
    assert!((x_east[1] - x_east[0] - 90.0).abs() < 1e-6);
    assert!((y_north[1] - y_north[0] - 45.0).abs() < 1e-6);
    assert!(x_east[1] <= 180.0 && x_east[0] >= -180.0);
    assert!(y_north[1] <= 90.0 && y_north[0] >= -90.0);

    for (lon, lat) in [
        (28.9784, 41.0082),   // Istanbul
        (-74.0060, 40.7128),  // New York
        (151.2093, -33.8688), // Sydney
        (139.6503, 35.6762),  // Tokyo
        (-179.8, 70.0),       // International Date Line
    ] {
        for zoom in [MapZoomLevel::Continental, MapZoomLevel::Regional] {
            let (x, y) = zoom.bounds(lon, lat);
            assert!(
                (x[0]..=x[1]).contains(&lon),
                "{zoom:?} lost longitude {lon}"
            );
            assert!((y[0]..=y[1]).contains(&lat), "{zoom:?} lost latitude {lat}");
        }
    }
}

#[test]
fn test_phase9_4_automation_monitor_switching_cycle() {
    let mut model = Model::new();
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;
    model.selected_monitor = 0;

    // Press ']' to switch to next monitor
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(']'),
        )),
    );
    assert_eq!(model.selected_monitor, 1);

    // Press '[' to switch back
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('['),
        )),
    );
    assert_eq!(model.selected_monitor, 0);

    // Context sharing: switch to Tab 1 retains selected_monitor
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(']'),
        )),
    );
    assert_eq!(model.selected_monitor, 1);

    model.active_tab = Tab::Monitors;
    assert_eq!(model.selected_monitor, 1);
}

#[test]
fn test_phase9_4_automation_curve_shape_interactive_control() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![
            named_monitor_config("mon-0", "Mi Monitor", 5, 60),
            named_monitor_config("mon-1", "LEN P24h-20", 8, 90),
        ],
    );
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;
    model.automation_focus = super::model::AutomationRegionFocus::Milestones;
    model.selected_monitor_milestone = 0;

    // Up from milestone 0 moves focus to Curve shape
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Up,
        )),
    );
    assert_eq!(
        model.automation_focus,
        super::model::AutomationRegionFocus::Curve
    );

    // Right on curve steps gamma by +0.05
    let prev_gamma = model.config.monitors[0].transition_gamma;
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Right,
        )),
    );
    let new_gamma = model.config.monitors[0].transition_gamma;
    assert!((new_gamma - (prev_gamma + 0.05)).abs() < 1e-4);

    // Down moves back to Milestones
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        )),
    );
    assert_eq!(
        model.automation_focus,
        super::model::AutomationRegionFocus::Milestones
    );
}

#[test]
fn test_phase9_4_brightness_range_instrument_and_stepping() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 7, 60)],
    );
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    model.monitor_control_index = 0; // MIN

    let prev_min = model.config.monitors[0].min_pct;
    // Right on MIN increases min by 1%
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Right,
        )),
    );
    assert_eq!(model.config.monitors[0].min_pct, prev_min + 1);

    // Left on MIN decreases min by 1%
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Left,
        )),
    );
    assert_eq!(model.config.monitors[0].min_pct, prev_min);
}

#[test]
fn test_phase9_4_1_applied_value_never_concatenates_or_leaves_stale_cells() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
    );
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    let set_applied = |model: &mut Model, value: Option<u8>| {
        model.status.as_mut().expect("fixture has status").monitors[0].last_applied_percent = value;
    };

    set_applied(&mut model, Some(100));
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let initial = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&initial, "Applied         ● 100%").is_some());

    set_applied(&mut model, Some(5));
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let after_100_to_5 = terminal.backend().buffer().clone();
    let applied_row = find_in_buffer(&after_100_to_5, "Applied")
        .expect("Applied row remains visible")
        .1;
    let applied_text = buffer_row(&after_100_to_5, applied_row);
    assert!(applied_text.contains("Applied         ● 5%"));
    assert!(!applied_text.contains("100%"));
    assert!(!applied_text.contains("1005%"));

    set_applied(&mut model, Some(44));
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let after_44 = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&after_44, "Applied         ● 44%").is_some());

    set_applied(&mut model, Some(5));
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let after_44_to_5 = terminal.backend().buffer().clone();
    let applied_row = find_in_buffer(&after_44_to_5, "Applied")
        .expect("Applied row remains visible")
        .1;
    let applied_text = buffer_row(&after_44_to_5, applied_row);
    assert!(applied_text.contains("Applied         ● 5%"));
    assert!(!applied_text.contains("445%"));

    set_applied(&mut model, Some(45));
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let after_45 = terminal.backend().buffer().clone();
    let applied_row = find_in_buffer(&after_45, "Applied")
        .expect("Applied row remains visible")
        .1;
    let applied_text = buffer_row(&after_45, applied_row);
    assert!(applied_text.contains("Applied         ● 45%"));
    assert!(!applied_text.contains("445%"));

    set_applied(&mut model, None);
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let after_unknown = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&after_unknown, "Applied         Unknown").is_some());
    assert!(!buffer_text(&after_unknown).contains("100Unknown"));

    // A malformed older IPC peer must not make an impossible value look valid.
    set_applied(&mut model, Some(101));
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let invalid_value = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&invalid_value, "Applied         Unknown").is_some());
    assert!(find_in_buffer(&invalid_value, "Applied         ● 101%").is_none());
}

#[test]
fn test_phase9_4_1_brightness_semantics_and_footer_are_unambiguous() {
    use super::command::{commands_for_footer, CommandId, UiCommandContext};

    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
    );
    let mut status = dummy_status(1);
    status.monitors[0].last_applied_percent = Some(44);
    status.monitors[0].override_percent = Some(60);
    model.status = Some(status);
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let text = buffer_text(&buffer);

    let applied = find_in_buffer(&buffer, "Applied         ● 44%").expect("applied row");
    let mode = find_in_buffer(&buffer, "Mode            Manual override · 60%").expect("mode row");
    assert_ne!(
        applied.1, mode.1,
        "measurement and mode must be separate rows"
    );
    assert!(find_in_buffer(&buffer, "Brightness").is_some());
    assert!(!text.contains("CURRENT BRIGHTNESS"));
    assert!(!text.contains("Actual output. Adjust the operating range below."));
    assert!(!text.contains("↑/↓ select handle"));
    // No target is shown until the policy engine has produced one.
    assert!(!text.contains("Target        "));
    model.monitor_targets_now = vec![super::model::TargetPreview {
        logical_id: String::from("mon-0"),
        solar_percent: 50,
        weather_percent: 42,
    }];
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let with_target = buffer_text(terminal.backend().buffer());
    assert!(with_target.contains("Target          42%  50% before weather"));
    assert!(with_target.contains("Applied         ● 44%"));

    let commands = commands_for_footer(&model);
    assert_eq!(commands.context, UiCommandContext::MonitorsDetail);
    assert!(commands
        .current_commands
        .iter()
        .any(|command| command.id == CommandId::SelectControlField));
    assert!(commands
        .current_commands
        .iter()
        .any(|command| command.id == CommandId::AdjustBrightnessRange));
    assert!(commands
        .current_commands
        .iter()
        .any(|command| command.id == CommandId::EditField));
    assert!(commands
        .current_commands
        .iter()
        .any(|command| command.id == CommandId::BackToList));
}

#[test]
fn test_phase9_4_1_stacked_range_navigation_exact_edit_and_validation() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
    );
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    model.monitor_control_index = 0;

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let initial = terminal.backend().buffer().clone();
    let min = find_in_buffer(&initial, "Minimum").expect("min control");
    let max = find_in_buffer(&initial, "Maximum").expect("max control");
    assert!(
        min.1 < max.1,
        "stacked controls must match vertical navigation"
    );
    assert_eq!(initial.get(min.0 - 2, min.1).symbol(), "❯");
    assert_eq!(initial.get(max.0 - 2, max.1).symbol(), " ");

    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Right)));
    assert_eq!(model.config.monitors[0].min_pct, 6);
    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Left)));
    assert_eq!(model.config.monitors[0].min_pct, 5);

    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Down)));
    assert_eq!(model.monitor_control_index, 1);
    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Right)));
    assert_eq!(model.config.monitors[0].max_pct, 61);
    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Left)));
    assert_eq!(model.config.monitors[0].max_pct, 60);
    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Up)));
    assert_eq!(model.monitor_control_index, 0);

    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Enter)));
    assert_eq!(model.input_mode, InputMode::Editing);
    assert_eq!(model.active_input_ref().expect("min input").value(), "5");
    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Char('9'))));
    assert_ne!(
        model.active_input_ref().expect("edited min input").value(),
        "5"
    );
    update::update(&mut model, Message::Key(KeyEvent::from(KeyCode::Esc)));
    assert_eq!(model.input_mode, InputMode::Normal);
    assert_eq!(model.form.monitor_inputs[0].0.value(), "5");
    assert_eq!(model.config.monitors[0].min_pct, 5);

    model.form.monitor_inputs[0].0 = tui_input::Input::default().with_value(String::from("61"));
    model.form.monitor_inputs[0].1 = tui_input::Input::default().with_value(String::from("60"));
    let error = model
        .form
        .validate_values(&model.config)
        .expect_err("exact entry must retain min <= max validation");
    assert!(error.contains("minimum"));
}

#[test]
fn test_phase9_4_1_range_track_and_control_focus_styles_agree() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
    );
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    let palette = model.config.tui.theme.palette();

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    model.monitor_control_index = 0;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let min_focused = terminal.backend().buffer().clone();
    let min = find_in_buffer(&min_focused, "Minimum").expect("min control");
    let max = find_in_buffer(&min_focused, "Maximum").expect("max control");
    let min_capsule = find_in_buffer(&min_focused, "‹  5%  ›").expect("min capsule");
    assert_eq!(min_focused.get(min.0 - 2, min.1).symbol(), "❯");
    assert_eq!(min_focused.get(max.0 - 2, max.1).symbol(), " ");
    assert_eq!(
        min_focused.get(min_capsule.0 + 3, min_capsule.1).bg,
        palette.accent
    );
    assert!(
        (0..min_focused.area.width).any(|x| {
            let cell = min_focused.get(x, min.1 - 1);
            cell.symbol() == "█" && cell.fg == palette.accent
        }),
        "the focused Minimum control must own the strong track handle"
    );

    model.monitor_control_index = 1;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let max_focused = terminal.backend().buffer().clone();
    let min = find_in_buffer(&max_focused, "Minimum").expect("min control");
    let max = find_in_buffer(&max_focused, "Maximum").expect("max control");
    let max_capsule = find_in_buffer(&max_focused, "‹  60%  ›").expect("max capsule");
    assert_eq!(max_focused.get(min.0 - 2, min.1).symbol(), " ");
    assert_eq!(max_focused.get(max.0 - 2, max.1).symbol(), "❯");
    assert_eq!(
        max_focused.get(max_capsule.0 + 3, max_capsule.1).bg,
        palette.accent
    );
    assert!(
        (0..max_focused.area.width).any(|x| {
            let cell = max_focused.get(x, max.1 - 2);
            cell.symbol() == "█" && cell.fg == palette.accent
        }),
        "the focused Maximum control must own the strong track handle"
    );

    model.monitor_pane_focus = super::model::MonitorPaneFocus::List;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let list_focused = terminal.backend().buffer().clone();
    let header = find_in_buffer(&list_focused, "Brightness range").expect("range header");
    assert!(
        !(0..list_focused.area.width).any(|x| list_focused.get(x, header.1 + 1).symbol() == "█"),
        "unfocused range must not falsely show an active handle"
    );

    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    model.monitor_control_index = 0;
    model.input_mode = InputMode::Editing;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let editing = terminal.backend().buffer().clone();
    let edit_capsule = find_in_buffer(&editing, "│ 5 │").expect("editing capsule");
    let edit_cell = editing.get(edit_capsule.0 + 2, edit_capsule.1);
    assert_eq!(edit_cell.bg, palette.secondary_accent);
    assert_ne!(edit_cell.bg, palette.warning);
}

#[test]
fn test_phase9_4_1_monitor_list_markers_names_and_shorter_redraw_are_clean() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![
            named_monitor_config("mon-0", "Mi Monitor", 5, 60),
            named_monitor_config("mon-1", "LEN P24h-20", 8, 70),
        ],
    );
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::List;

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let list_focused = terminal.backend().buffer().clone();
    let list_text = buffer_text(&list_focused);
    assert!(list_text.contains("Mi Monitor"));
    assert!(list_text.contains("Lenovo P24h-20"));
    assert!(!list_text.contains("LEN P24h-20"));
    assert_eq!(list_text.matches('❯').count(), 1);
    assert_eq!(list_text.matches('▸').count(), 0);

    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let detail_focused = terminal.backend().buffer().clone();
    let detail_text = buffer_text(&detail_focused);
    assert_eq!(detail_text.matches("› Mi Monitor").count(), 1);
    assert!(!detail_text.contains("❯ Mi Monitor"));
    assert_eq!(detail_text.matches('▸').count(), 0);

    model.monitor_pane_focus = super::model::MonitorPaneFocus::List;
    model.config.monitors[0].selector.model = Some(String::from("東京 Ultra-Wide Display"));
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let long_name = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&long_name, "…").is_some());

    model.config.monitors[0].selector.model = Some(String::from("Mi"));
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let short_name = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&short_name, "Mi").is_some());
    assert!(find_in_buffer(&short_name, "東京 Ultra-Wide").is_none());
}

#[test]
fn test_phase9_4_1_full_header_reserves_a_breathing_row() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
    );
    model.config.tui.show_logo = true;
    model.config.location.city = String::from("Istanbul, TR");
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;

    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    assert!(buffer_row(&buffer, 0).contains("____"));
    assert!(buffer_row(&buffer, 4).contains("|____"));
    assert!(buffer_row(&buffer, 5).trim().is_empty());
    assert!(buffer_row(&buffer, 6).contains("Live"));
    assert!(buffer_row(&buffer, 6).contains("25%"));
    assert!(!buffer_row(&buffer, 6).contains("Sun +"));
    assert!(buffer_row(&buffer, 7).trim().is_empty());
    assert!(buffer_row(&buffer, 8).contains("1 Monitors"));
    assert!(buffer_row(&buffer, 9).contains('━'));

    let mut compact_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let compact = compact_terminal.draw(|f| ui::ui(f, &mut model));
    assert!(compact.is_ok());
}

#[test]
fn test_phase9_4_curve_upper_bound_uses_config_authority() {
    let mut model = Model::new();
    model.config.monitors = vec![crate::config::MonitorConfig {
        logical_id: String::from("mon-0"),
        transition_gamma: crate::config::MAX_TRANSITION_GAMMA - 0.05,
        ..Default::default()
    }];
    model.form = super::form::FormState::new(&model.config);
    model.status = Some(dummy_status(1));

    model.step_monitor_curve(0.05);
    assert!(
        (model.config.monitors[0].transition_gamma - crate::config::MAX_TRANSITION_GAMMA).abs()
            < f64::EPSILON
    );

    // Repeating the key at the cap is a no-op rather than creating an invalid config.
    model.step_monitor_curve(0.05);
    assert!(
        (model.config.monitors[0].transition_gamma - crate::config::MAX_TRANSITION_GAMMA).abs()
            < f64::EPSILON
    );

    model.form.monitor_curve_inputs[0] =
        tui_input::Input::default().with_value(String::from("4.01"));
    let error = model
        .form
        .validate_values(&model.config)
        .expect_err("exact entry above config cap must fail");
    assert!(error.contains("0 < gamma <= 4"));
}

#[test]
fn test_phase9_4_shared_policy_produces_monitor_specific_targets() {
    let policy = crate::config::SolarPolicyConfig {
        twilight_elevation_start: -6.0,
        day_elevation_full: 20.0,
        use_adaptive_zenith: false,
        ..Default::default()
    };
    let monitors = vec![
        crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            min_pct: 7,
            max_pct: 60,
            transition_gamma: 0.5,
            ..Default::default()
        },
        crate::config::MonitorConfig {
            logical_id: String::from("mon-1"),
            min_pct: 15,
            max_pct: 90,
            transition_gamma: 2.0,
            ..Default::default()
        },
    ];
    let location = crate::solar::Location::from_timezone_name(41.0082, 28.9784, "Europe/Istanbul")
        .expect("valid test location");
    let now = chrono::Utc
        .with_ymd_and_hms(2026, 9, 10, 9, 0, 0)
        .single()
        .expect("valid fixed instant");

    let output = crate::policy::compute_policy_for_elevation(
        &crate::policy::PolicyContext {
            now_utc: now,
            location: &location,
            config: &policy,
            monitors: &monitors,
            weather_multiplier: None,
        },
        7.0,
    )
    .expect("shared policy accepts valid monitor fixtures");

    assert_eq!(output.targets.len(), 2);
    assert_ne!(output.targets[0].percent, output.targets[1].percent);
    assert!(
        (output.targets[0].effective_daylight_factor - output.targets[1].effective_daylight_factor)
            .abs()
            > f64::EPSILON
    );
}

#[test]
fn test_phase9_4_automation_switch_updates_context_curve_and_target() {
    let mut model = Model::new();
    model.config.monitors = vec![
        crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            min_pct: 7,
            max_pct: 60,
            transition_gamma: 0.5,
            ..Default::default()
        },
        crate::config::MonitorConfig {
            logical_id: String::from("mon-1"),
            min_pct: 15,
            max_pct: 90,
            transition_gamma: 2.0,
            ..Default::default()
        },
    ];
    model.form = super::form::FormState::new(&model.config);
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Limits;

    let time = FixedOffset::east_opt(0)
        .unwrap()
        .with_ymd_and_hms(2026, 9, 10, 9, 0, 0)
        .single()
        .unwrap();
    model.monitor_milestones = vec![
        MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![MonitorMilestone {
                milestone: AutomationMilestone::Rise50,
                base_time_local: time,
                adjusted_time_local: time,
                target_percent: 22,
                minutes_offset: 0,
            }],
        },
        MonitorMilestoneSchedule {
            logical_id: String::from("mon-1"),
            milestones: vec![MonitorMilestone {
                milestone: AutomationMilestone::Rise50,
                base_time_local: time,
                adjusted_time_local: time,
                target_percent: 78,
                minutes_offset: 0,
            }],
        },
    ];

    let mut first_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    first_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let first = first_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&first, "mon-0").is_some());
    assert!(find_in_buffer(&first, "0.50").is_some());
    assert!(find_in_buffer(&first, "22%").is_some());

    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char(']'),
        )),
    );
    assert_eq!(model.selected_monitor, 1);

    let mut second_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    second_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let second = second_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&second, "mon-1").is_some());
    assert!(find_in_buffer(&second, "2.00").is_some());
    assert!(find_in_buffer(&second, "78%").is_some());
}

#[test]
fn test_phase9_4_range_repeat_defers_schedule_recompute_and_marks_preview() {
    let mut model = Model::new();
    model.config.monitors = vec![crate::config::MonitorConfig {
        logical_id: String::from("mon-0"),
        min_pct: 7,
        max_pct: 60,
        ..Default::default()
    }];
    model.form = super::form::FormState::new(&model.config);
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;

    let time = FixedOffset::east_opt(0)
        .unwrap()
        .with_ymd_and_hms(2026, 9, 10, 9, 0, 0)
        .single()
        .unwrap();
    model.monitor_milestones = vec![MonitorMilestoneSchedule {
        logical_id: String::from("mon-0"),
        milestones: vec![MonitorMilestone {
            milestone: AutomationMilestone::Rise50,
            base_time_local: time,
            adjusted_time_local: time,
            target_percent: 33,
            minutes_offset: 0,
        }],
    }];
    let cached_schedule = model.monitor_milestones.clone();

    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;
    model.monitor_control_index = 0;
    super::update::update(
        &mut model,
        super::update::Message::Key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Right,
        )),
    );

    assert_eq!(model.config.monitors[0].min_pct, 8);
    assert!(model.monitor_policy_preview_pending());
    assert_eq!(model.monitor_milestones, cached_schedule);

    // The periodic refresh path must also leave the cached schedule intact
    // until the debounced persistence cycle settles.
    model.refresh_monitor_milestones_if_needed();
    assert_eq!(model.monitor_milestones, cached_schedule);

    model.active_tab = Tab::Limits;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    assert!(
        find_in_buffer(&buffer, "Target updating…").is_some(),
        "{}",
        buffer_text(&buffer)
    );
}

#[test]
fn test_phase9_4_location_renderer_uses_location_centric_globe() {
    let mut model = Model::new();
    model.active_tab = Tab::Location;
    model.config.location.city = String::from("Istanbul");
    model.config.location.latitude = 41.0082;
    model.config.location.longitude = 28.9784;

    let mut wide_terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    wide_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = wide_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer, "Earth").is_some());
    assert!(find_in_buffer(&buffer, "41.00820° N").is_some());
    assert!(find_in_buffer(&buffer, "28.97840° E").is_some());

    // Compact Location keeps the factual fields but does not attempt to
    // squeeze an illegible globe into a minimal terminal.
    let mut compact_terminal = Terminal::new(TestBackend::new(50, 14)).unwrap();
    compact_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let compact_buffer = compact_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&compact_buffer, "Latitude").is_some());
    assert!(find_in_buffer(&compact_buffer, "Timezone").is_some());
}

#[test]
fn test_phase9_4_curve_focus_footer_and_help_are_truthful() {
    use super::command::{
        commands_for_footer, commands_for_workspace, CommandId, UiCommandContext,
    };

    let mut model = Model::new();
    model.active_tab = Tab::Limits;
    model.automation_focus = super::model::AutomationRegionFocus::Curve;

    let commands = commands_for_footer(&model);
    assert_eq!(commands.context, UiCommandContext::AutomationCurve);
    assert!(commands
        .current_commands
        .iter()
        .any(|command| command.id == CommandId::AdjustGamma));
    assert!(!commands
        .current_commands
        .iter()
        .any(|command| command.id == CommandId::AdjustMilestoneOffset));

    model.show_help = true;
    let help_commands = commands_for_workspace(&model);
    assert_eq!(help_commands.context, UiCommandContext::AutomationCurve);
    assert!(help_commands
        .current_commands
        .iter()
        .any(|command| command.action == "Adjust gamma by 0.05"));
}

#[test]
fn test_phase9_4_curve_focus_hides_milestone_only_inspector() {
    let mut model = Model::new();
    model.config.monitors = vec![crate::config::MonitorConfig {
        logical_id: String::from("mon-0"),
        min_pct: 7,
        max_pct: 60,
        transition_gamma: 0.5,
        ..Default::default()
    }];
    model.form = super::form::FormState::new(&model.config);
    model.status = Some(dummy_status(1));
    model.active_tab = Tab::Limits;
    model.automation_focus = super::model::AutomationRegionFocus::Curve;

    let time = FixedOffset::east_opt(0)
        .unwrap()
        .with_ymd_and_hms(2026, 9, 10, 9, 0, 0)
        .single()
        .unwrap();
    model.monitor_milestones = vec![MonitorMilestoneSchedule {
        logical_id: String::from("mon-0"),
        milestones: vec![MonitorMilestone {
            milestone: AutomationMilestone::Rise50,
            base_time_local: time,
            adjusted_time_local: time,
            target_percent: 22,
            minutes_offset: 0,
        }],
    }];

    let mut curve_terminal = Terminal::new(TestBackend::new(85, 26)).unwrap();
    curve_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let curve_buffer = curve_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&curve_buffer, "Gamma").is_some());
    assert!(find_in_buffer(&curve_buffer, "±1 min").is_none());
    assert!(find_in_buffer(&curve_buffer, "r Reset").is_none());

    model.automation_focus = super::model::AutomationRegionFocus::Milestones;
    let mut milestone_terminal = Terminal::new(TestBackend::new(85, 26)).unwrap();
    milestone_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let milestone_buffer = milestone_terminal.backend().buffer().clone();
    assert!(find_in_buffer(&milestone_buffer, "±1 min").is_some());
    assert!(find_in_buffer(&milestone_buffer, "r Reset").is_some());
}

#[test]
fn test_phase9_4_selected_monitor_identity_survives_status_reorder() {
    let mut model = Model::new();
    model.config.monitors = vec![
        crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            min_pct: 7,
            max_pct: 60,
            transition_gamma: 0.5,
            ..Default::default()
        },
        crate::config::MonitorConfig {
            logical_id: String::from("mon-1"),
            min_pct: 15,
            max_pct: 90,
            transition_gamma: 2.0,
            ..Default::default()
        },
    ];
    model.form = super::form::FormState::new(&model.config);
    model.status = Some(dummy_status(2));
    model.selected_monitor = 1;
    model.clamp_monitor_selection();
    assert_eq!(model.selected_monitor_logical_id(), Some("mon-1"));

    let mut reordered = dummy_status(2);
    reordered.monitors.swap(0, 1);
    update::update(
        &mut model,
        Message::Ipc(IpcEvent::Status(Box::new(reordered))),
    );

    assert_eq!(model.selected_monitor, 0);
    assert_eq!(model.selected_monitor_logical_id(), Some("mon-1"));
    assert_eq!(model.monitor_list_state.selected(), Some(0));

    model.step_monitor_curve(0.05);
    assert!((model.config.monitors[0].transition_gamma - 0.5).abs() < f64::EPSILON);
    assert!((model.config.monitors[1].transition_gamma - 2.05).abs() < f64::EPSILON);
    assert_eq!(model.form.monitor_curve_inputs[1].value(), "2.05");
}

#[test]
fn test_phase9_4_removed_selected_monitor_uses_nearest_configured_context() {
    let mut model = Model::new();
    model.config.monitors = (0..3)
        .map(|index| crate::config::MonitorConfig {
            logical_id: format!("mon-{index}"),
            ..Default::default()
        })
        .collect();
    model.form = super::form::FormState::new(&model.config);
    model.status = Some(dummy_status(3));
    model.selected_monitor = 1;
    model.clamp_monitor_selection();
    assert_eq!(model.selected_monitor_logical_id(), Some("mon-1"));

    model.config.monitors.remove(1);
    model.form = super::form::FormState::new(&model.config);
    let mut without_selected = dummy_status(3);
    without_selected.monitors.remove(1);
    model.status = Some(without_selected);
    model.clamp_monitor_selection();

    assert_eq!(model.selected_monitor, 1);
    assert_eq!(model.selected_monitor_logical_id(), Some("mon-2"));
    assert_eq!(model.monitor_list_state.selected(), Some(1));
}

// ══════════════════════════════════════════════════════════════════════════
// UX pass: isolation, policy previews, unified grammar and motion policy
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_policy_previews_stay_inside_the_configured_range_and_morph_only_in_full_effects() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 12, 64)],
    );
    model.motion.level = crate::config::MotionLevel::Instrument;
    model.refresh_monitor_milestones();

    let curve = model.monitor_curves.first().expect("curve preview");
    assert_eq!(curve.logical_id, "mon-0");
    assert_eq!(
        curve.samples.len(),
        (24 * 60 / super::model::CURVE_SAMPLE_MINUTES + 1) as usize
    );
    assert!(curve
        .samples
        .iter()
        .all(|value| (12.0..=64.0).contains(value)));
    let target = model.monitor_targets_now.first().expect("target preview");
    assert!((12..=64).contains(&target.solar_percent));
    assert_eq!(target.solar_percent, target.weather_percent);

    // A different curve shape changes the sampled curve and eases into it.
    model.config.monitors[0].transition_gamma = 2.5;
    model.refresh_monitor_milestones();
    let changed = model.monitor_curves[0].samples
        != model
            .curve_morph_from
            .as_ref()
            .map_or_else(Vec::new, |from| from.samples.clone());
    if changed && model.curve_morph_from.is_some() {
        assert!(model.motion.curve_morph_phase(Instant::now()).is_some());
    }

    // Off never animates.
    model.motion.level = crate::config::MotionLevel::Off;
    model.motion.active_transient = None;
    model.config.monitors[0].transition_gamma = 0.3;
    model.refresh_monitor_milestones();
    assert!(model.motion.curve_morph_phase(Instant::now()).is_none());
}

#[test]
fn test_tab_slide_and_globe_arrival_follow_the_effects_level() {
    let mut model = Model::new();
    model.motion.level = crate::config::MotionLevel::Instrument;
    model.switch_to_tab(Tab::Location);
    let now = Instant::now();
    assert!(model.motion.tab_slide_position(now).is_some());
    assert!(model.motion.globe_rotation_phase(now).is_some());

    let mut reduced = Model::new();
    reduced.motion.level = crate::config::MotionLevel::Reduced;
    reduced.switch_to_tab(Tab::Location);
    let now = Instant::now();
    assert!(reduced.motion.tab_slide_position(now).is_none());
    assert!(reduced.motion.globe_rotation_phase(now).is_none());

    let mut off = Model::new();
    off.motion.level = crate::config::MotionLevel::Off;
    off.switch_to_tab(Tab::Weather);
    assert!(!off.motion.needs_animation_frame(Instant::now()));
}

#[test]
fn test_unconfigured_daemon_monitor_never_shows_invented_limits() {
    let mut model = Model::new();
    model.status = Some(dummy_status(1));
    model.daemon_connection = super::DaemonConnection::Connected;
    model.active_tab = Tab::Monitors;
    model.monitor_pane_focus = super::model::MonitorPaneFocus::Detail;

    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let text = buffer_text(terminal.backend().buffer());
    assert!(text.contains("Not in configuration"), "{text}");
    assert!(!text.contains("Minimum"), "{text}");
    assert!(!text.contains("Gamma"), "{text}");

    model.active_tab = Tab::Limits;
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let automation = buffer_text(terminal.backend().buffer());
    assert!(automation.contains("Not configured"), "{automation}");
}

#[test]
fn test_location_globe_renders_filled_land_and_a_centered_marker() {
    let mut model = Model::new();
    model.config.location.city = String::from("Istanbul, TR");
    model.config.location.latitude = 41.0138;
    model.config.location.longitude = 28.9497;
    model.config.location.timezone = String::from("Europe/Istanbul");
    model.form.refresh_from_config(&model.config);
    model.active_tab = Tab::Location;
    model.motion.level = crate::config::MotionLevel::Off;

    let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let (earth_x, earth_y, _) = find_in_buffer(&buffer, "╭ Earth").expect("earth panel");

    let braille = (earth_y..buffer.area.height)
        .flat_map(|y| (earth_x..buffer.area.width).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            buffer
                .get(x, y)
                .symbol()
                .chars()
                .next()
                .is_some_and(|c| ('\u{2801}'..='\u{28FF}').contains(&c))
        })
        .count();
    assert!(
        braille > 300,
        "globe should be filled, found {braille} braille cells"
    );

    // The configured location is the projection centre, so its marker sits
    // near the middle of the Earth panel.
    let marker = (earth_y + 1..buffer.area.height - 2)
        .flat_map(|y| (earth_x..buffer.area.width).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let cell = buffer.get(x, y);
            cell.symbol() == "●" && cell.fg == model.config.tui.theme.palette().accent
        })
        .expect("location marker");
    let panel_center_x = earth_x + (buffer.area.width - earth_x) / 2;
    assert!(
        marker.0.abs_diff(panel_center_x) <= 3,
        "marker {marker:?} vs {panel_center_x}"
    );
}

#[test]
fn test_background_preview_refresh_applies_latest_generation_only() {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![named_monitor_config("mon-0", "Mi Monitor", 10, 70)],
    );
    model.monitor_milestones.clear();
    model.monitor_curves.clear();

    // Two requests: only the second (latest) generation may be applied.
    model.request_preview_refresh();
    let first = model.preview_generation;
    model.request_preview_refresh();
    let second = model.preview_generation;
    assert!(second > first);
    assert!(
        model.monitor_curves.is_empty(),
        "the input thread is not blocked"
    );

    let deadline = Instant::now() + Duration::from_secs(20);
    while model.preview_job.is_some() && Instant::now() < deadline {
        model.poll_preview_refresh();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(model.preview_job.is_none());
    assert!(!model.monitor_milestones.is_empty());
    assert_eq!(model.monitor_curves.len(), 1);
    assert!(!model.monitor_policy_preview_pending());
}

// ══════════════════════════════════════════════════════════════════════════
// Keyboard spatial consistency: horizontal rows use ←/→, vertical lists ↑/↓
// ══════════════════════════════════════════════════════════════════════════

fn press(model: &mut Model, code: KeyCode, modifiers: KeyModifiers) {
    update::update(
        model,
        Message::Key(key_event(code, modifiers, KeyEventKind::Press)),
    );
}

fn two_monitor_model() -> Model {
    let mut model = Model::new();
    configure_monitor_fixture(
        &mut model,
        vec![
            named_monitor_config("mon-0", "Mi Monitor", 5, 60),
            named_monitor_config("mon-1", "LEN P24h-20", 8, 90),
        ],
    );
    model.status = Some(dummy_status(2));
    model.daemon_connection = super::DaemonConnection::Connected;
    model
}

#[test]
fn test_tab_bar_takes_focus_and_uses_left_right() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Location;
    model.active_setting = 0;

    // ↑ from the first field reaches the tab bar.
    press(&mut model, KeyCode::Up, KeyModifiers::NONE);
    assert!(model.tabs_focused);

    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let (_, tab_row, _) = find_in_buffer(&buffer, "3 Location").unwrap();
    assert_eq!(buffer.get(0, tab_row).symbol(), "❯");
    let (x, y, _) = find_in_buffer(&buffer, "3 Location").unwrap();
    assert_eq!(buffer.get(x, y).bg, model.config.tui.theme.palette().accent);
    assert!(find_in_buffer(&buffer, "Workspace").is_some());
    // The form no longer shows a cursor while the tab bar has focus.
    assert!(find_in_buffer(&buffer, "❯ City").is_none());

    press(&mut model, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(model.active_tab, Tab::Weather);
    assert!(
        model.tabs_focused,
        "focus stays on the tab bar while moving along it"
    );
    press(&mut model, KeyCode::Left, KeyModifiers::NONE);
    press(&mut model, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(model.active_tab, Tab::Limits);

    press(&mut model, KeyCode::Down, KeyModifiers::NONE);
    assert!(!model.tabs_focused);
    assert_eq!(model.active_tab, Tab::Limits);
}

#[test]
fn test_browser_style_workspace_keys() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Location;

    press(&mut model, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(model.active_tab, Tab::Weather);
    press(&mut model, KeyCode::Tab, KeyModifiers::SHIFT);
    assert_eq!(model.active_tab, Tab::Location);
    press(&mut model, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(model.active_tab, Tab::Limits);
    press(&mut model, KeyCode::Tab, KeyModifiers::CONTROL);
    assert_eq!(model.active_tab, Tab::Location);
    press(
        &mut model,
        KeyCode::Tab,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!(model.active_tab, Tab::Limits);
    press(&mut model, KeyCode::PageDown, KeyModifiers::CONTROL);
    assert_eq!(model.active_tab, Tab::Location);
    press(&mut model, KeyCode::PageUp, KeyModifiers::CONTROL);
    assert_eq!(model.active_tab, Tab::Limits);
    // Plain PgUp stays a list key.
    press(&mut model, KeyCode::PageUp, KeyModifiers::NONE);
    assert_eq!(model.active_tab, Tab::Limits);

    let help = super::command::commands_for_workspace(&model);
    assert!(help
        .global_commands
        .iter()
        .any(|command| command.keys == "Tab / Shift+Tab"));
}

#[test]
fn test_automation_monitor_chips_switch_with_left_right() {
    let mut model = two_monitor_model();
    model.switch_to_tab(Tab::Limits);
    assert_eq!(
        model.automation_focus,
        super::model::AutomationRegionFocus::Selector,
        "with several monitors Automation opens on the monitor chips"
    );

    let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let (_, row, _) = find_in_buffer(&buffer, "Mi Monitor").unwrap();
    assert!(buffer_row(&buffer, row).contains("Mi Monitor"));
    assert!(buffer_row(&buffer, row).contains("›"), "{buffer:?}");
    assert!(find_in_buffer(&buffer, "Brightness target").is_some());
    assert!(find_in_buffer(&buffer, "Output").is_some());
    let footer = buffer_row(&buffer, 31);
    assert!(footer.contains("Monitor"), "{footer}");

    press(&mut model, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(model.selected_monitor, 1);
    press(&mut model, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(model.selected_monitor, 0);

    press(&mut model, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        model.automation_focus,
        super::model::AutomationRegionFocus::Curve
    );
    press(&mut model, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        model.automation_focus,
        super::model::AutomationRegionFocus::Selector
    );
    press(&mut model, KeyCode::Up, KeyModifiers::NONE);
    assert!(model.tabs_focused);
}

#[test]
fn test_automation_three_region_layout_survives_the_chrome_gutter() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Limits;
    model.motion.level = crate::config::MotionLevel::Off;
    let mut terminal = Terminal::new(TestBackend::new(114, 32)).unwrap();

    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let (cycle_x, cycle_y, _) = find_in_buffer(&buffer, "Today's light cycle").expect("cycle");
    let (schedule_x, schedule_y, _) = find_in_buffer(&buffer, "Schedule").expect("schedule");

    assert_eq!(
        cycle_y, schedule_y,
        "schedule should remain beside the cycle"
    );
    assert!(
        schedule_x > cycle_x,
        "schedule should be the right-hand region"
    );
}

#[test]
fn test_vertical_monitor_list_uses_up_down_and_right_enters_controls() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Monitors;
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    assert!(!model.monitor_selector_horizontal);

    press(&mut model, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(model.selected_monitor, 1);
    press(&mut model, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(
        model.monitor_pane_focus,
        super::model::MonitorPaneFocus::Detail
    );
    assert_eq!(model.selected_monitor, 1, "→ never changes the selection");

    press(&mut model, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        model.monitor_pane_focus,
        super::model::MonitorPaneFocus::List
    );
    press(&mut model, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(model.selected_monitor, 0);
    press(&mut model, KeyCode::Up, KeyModifiers::NONE);
    assert!(
        model.tabs_focused,
        "↑ from the first monitor reaches the tab bar"
    );
}

#[test]
fn test_narrow_monitor_switcher_uses_left_right() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Monitors;
    let mut terminal = Terminal::new(TestBackend::new(60, 18)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    assert!(model.monitor_selector_horizontal);
    let commands = super::command::commands_for_footer(&model);
    assert_eq!(commands.current_commands[0].keys, "← →");

    press(&mut model, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(model.selected_monitor, 1);
    press(&mut model, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        model.monitor_pane_focus,
        super::model::MonitorPaneFocus::Detail
    );
    press(&mut model, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(
        model.monitor_pane_focus,
        super::model::MonitorPaneFocus::List
    );
}

#[test]
fn test_settings_focus_moves_straight_down_one_column() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Settings;
    model.active_setting = 0;
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();

    let mut previous: Option<(u16, u16)> = None;
    for step in 0..super::model::settings_index::COUNT {
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
fn test_footer_keeps_help_and_quit_when_space_is_short() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Monitors;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let footer = buffer_row(terminal.backend().buffer(), 23);
    assert!(footer.contains("? Help"), "{footer}");
    assert!(footer.contains("q Quit"), "{footer}");
}

#[test]
fn test_monitors_down_from_tab_bar_returns_to_the_monitor_list() {
    let mut model = two_monitor_model();
    model.active_tab = Tab::Monitors;
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();

    press(&mut model, KeyCode::Down, KeyModifiers::NONE);
    press(&mut model, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(
        model.monitor_pane_focus,
        super::model::MonitorPaneFocus::Detail
    );
    press(&mut model, KeyCode::Up, KeyModifiers::NONE);
    assert!(
        model.tabs_focused,
        "↑ from the first range row reaches the tabs"
    );

    press(&mut model, KeyCode::Down, KeyModifiers::NONE);
    assert!(!model.tabs_focused);
    assert_eq!(
        model.monitor_pane_focus,
        super::model::MonitorPaneFocus::List,
        "↓ from the tabs lands on the monitor list"
    );
    assert_eq!(model.selected_monitor, 1, "the selection is kept");
}
