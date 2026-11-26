    use crate::tui::InputMode;
    use crossterm::event::{KeyCode, KeyEvent};
    use ratatui::{backend::TestBackend, Terminal};
    use std::time::Instant;

    use crate::config::Config;
    use crate::discovery::{
        BackendStatus, BackendStatusKind, DdcMonitorDiscovery, DiscoveryBackends, DiscoveryReport,
        DiscoverySummary, PhysicalIdentityStatus,
    };
    use crate::tui::form::FormState;
    use crate::tui::model::{
        AutomationRegionFocus, DaemonConnection, MonitorDiscoveryState, MonitorWorkspaceState,
    };
    use crate::tui::test_support::{
        buffer_row, buffer_text, configure_monitor_fixture, dummy_status, find_in_buffer,
        named_monitor_config,
    };
    use crate::tui::ui;
    use crate::tui::update::{self, Message};
    use crate::tui::{Model, MonitorPaneFocus, Tab};

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
        model.form = FormState::new(&model.config);
        model.selected_monitor = 0;
        model.selected_monitor_id = None;
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
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;
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
    fn test_monitors_skip_reasons_are_shown_in_detail_header() {
        let styles = crate::config::ThemeId::default().styles();
        let now = 1_000_u64;

        let mut contested = dummy_status(1).monitors.remove(0);
        contested.contested = true;
        contested.contested_until_epoch_s = Some(now + 120);
        let (_, text, _) =
            crate::tui::ui::monitors::detail::monitor_state(Some(&contested), now, &styles);
        assert_eq!(text, "Contested · 2m");

        let mut manual_cap = dummy_status(1).monitors.remove(0);
        manual_cap.manual_deferred_until_epoch_s = Some(now + 5);
        let (_, text, _) =
            crate::tui::ui::monitors::detail::monitor_state(Some(&manual_cap), now, &styles);
        assert_eq!(text, "Manual cap · 5s");

        let mut budget = dummy_status(1).monitors.remove(0);
        budget.writes = Some(crate::ipc::WriteBudgetStatus {
            writes_total: 12,
            window_used: 12,
            window_max: 12,
            window_seconds: 600,
            deferred: true,
        });
        let (_, text, _) =
            crate::tui::ui::monitors::detail::monitor_state(Some(&budget), now, &styles);
        assert_eq!(text, "Write budget deferred");
    }

    #[test]
    fn test_monitors_workspace_compact_and_minimal_viewports() {
        // Compact: 60x18
        {
            let backend = TestBackend::new(60, 18);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.status = Some(dummy_status(2));
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
            model.active_tab = Tab::Monitors;

            let res = terminal.draw(|f| ui::ui(f, &mut model));
            assert!(res.is_ok());
        }
    }

    #[test]
    fn test_monitors_workspace_non_amber_themes() {
        for theme in [
            crate::config::ThemeId::new("nord"),
            crate::config::ThemeId::new("hackergreen"),
        ] {
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.theme = theme.clone();
            model.status = Some(dummy_status(1));
            model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Must show uppercase DDC, never Rust Debug 'Ddc'
        assert!(find_in_buffer(&buffer, "DDC").is_some());
    }

    #[test]
    fn monitor_workspace_distinguishes_discovery_and_no_hardware_states() {
        let mut model = Model::new();
        reset_to_empty_monitor_config(&mut model);
        model.status = Some(dummy_status(0));
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Disconnected;

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
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;

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
        model.automation_focus = AutomationRegionFocus::Curve;
        let mut automation_terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        automation_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let automation_buffer = automation_terminal.backend().buffer().clone();
        // Automation keeps the selected monitor compact but never regresses to a
        // raw logical ID when friendly configuration metadata exists.
        let (_, selector_row, _) =
            find_in_buffer(&automation_buffer, "Lenovo P24h").expect("selector");
        assert!(buffer_row(&automation_buffer, selector_row).contains("Monitor"));
        assert!(find_in_buffer(&automation_buffer, "2 / 2").is_some());
        assert!(find_in_buffer(&automation_buffer, "mon-1").is_none());
    }

    #[test]
    fn shared_monitor_selector_uses_configured_ids_while_daemon_records_are_empty() {
        let mut model = Model::new();
        reset_to_empty_monitor_config(&mut model);
        model.config.monitors = discovery_report_with_ddc_monitors(2).importable_monitor_configs();
        model.form = FormState::new(&model.config);
        let first_name = model.config.monitors[0]
            .selector
            .model
            .clone()
            .expect("fixture config has a friendly model name");
        let second_id = model.config.monitors[1].logical_id.clone();
        let second_name =
            crate::tui::ui::monitor_selector::monitor_display_name(&model, &second_id);

        let mut status = dummy_status(0);
        status.configured_monitors = 2;
        model.status = Some(status);
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = MonitorPaneFocus::Detail;
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
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.selected_monitor = 1;
        model.monitor_pane_focus = MonitorPaneFocus::Detail;

        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        // With detail focused, the list keeps a retained-selection marker.
        assert!(find_in_buffer(&buffer, "› mon-1").is_some());
        assert!(find_in_buffer(&buffer, "❯ mon-1").is_none());
    }

    #[test]
    fn test_phase5_5_2_selected_monitor_legibility_and_contrast_across_themes() {
        for theme in [
            crate::config::ThemeId::new("amber"),
            crate::config::ThemeId::new("nord"),
            crate::config::ThemeId::new("hackergreen"),
            crate::config::ThemeId::new("tokyonight"),
            crate::config::ThemeId::new("grayscale"),
            crate::config::ThemeId::new("commodore64"),
        ] {
            let backend = TestBackend::new(85, 26);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.theme = theme.clone();
            model.status = Some(dummy_status(2));
            model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
            model.active_tab = Tab::Monitors;
            model.selected_monitor = 0;
            model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::List;

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
            model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer_detail = terminal.backend().buffer().clone();

            let retained_marker =
                find_in_buffer(&buffer_detail, "› mon-0").expect("retained marker");
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
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
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
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
        model.monitor_control_index = 0; // Minimum brightness
        model.motion = crate::tui::motion::UiMotionState::new_at(now);

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
        let (_, _, theme_value) = find_in_buffer(&buffer, &model.config.tui.theme.name()).unwrap();
        assert_eq!(theme_value.bg, palette.accent);

        // 2. Animation rate focused.
        model.active_setting = crate::tui::model::settings_index::REFRESH_RATE;
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
    fn test_phase9_3_1_focus_salience_cell_style_and_editing_semantics() {
        let mut model = Model::new();
        model.active_tab = Tab::Settings;
        model.active_setting = 0; // Focus on Theme

        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buf = terminal.backend().buffer().clone();

        // Find the focused Theme value.
        let (vx, vy, _) = find_in_buffer(&buf, &model.config.tui.theme.name()).unwrap();
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
        model.active_setting = crate::tui::model::settings_index::REFRESH_RATE;
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
    fn test_phase9_4_brightness_range_instrument_and_stepping() {
        let mut model = Model::new();
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 7, 60)],
        );
        model.status = Some(dummy_status(1));
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
        model.monitor_control_index = 0; // MIN

        let prev_min = model.config.monitors[0].min_pct;
        // Right on MIN increases min by 1%
        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Right,
            )),
        );
        assert_eq!(model.config.monitors[0].min_pct, prev_min + 1);

        // Left on MIN decreases min by 1%
        crate::tui::update::update(
            &mut model,
            crate::tui::update::Message::Key(crossterm::event::KeyEvent::from(
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
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

        let set_applied = |model: &mut Model, value: Option<u8>| {
            model.status.as_mut().expect("fixture has status").monitors[0].last_applied_percent =
                value;
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
        use crate::tui::command::{commands_for_footer, CommandId, UiCommandContext};

        let mut model = Model::new();
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
        );
        let mut status = dummy_status(1);
        status.monitors[0].last_applied_percent = Some(44);
        status.monitors[0].override_percent = Some(60);
        model.status = Some(status);
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let text = buffer_text(&buffer);

        let applied = find_in_buffer(&buffer, "Applied         ● 44%").expect("applied row");
        let mode =
            find_in_buffer(&buffer, "Mode            Manual override · 60%").expect("mode row");
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
        model.monitor_targets_now = vec![crate::tui::model::TargetPreview {
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
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
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
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
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

        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::List;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let list_focused = terminal.backend().buffer().clone();
        let header = find_in_buffer(&list_focused, "Brightness range").expect("range header");
        assert!(
            !(0..list_focused.area.width)
                .any(|x| list_focused.get(x, header.1 + 1).symbol() == "█"),
            "unfocused range must not falsely show an active handle"
        );

        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
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
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::List;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let list_focused = terminal.backend().buffer().clone();
        let list_text = buffer_text(&list_focused);
        assert!(list_text.contains("Mi Monitor"));
        assert!(list_text.contains("Lenovo P24h-20"));
        assert!(!list_text.contains("LEN P24h-20"));
        assert_eq!(list_text.matches('❯').count(), 1);
        assert_eq!(list_text.matches('▸').count(), 0);

        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;
        terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let detail_focused = terminal.backend().buffer().clone();
        let detail_text = buffer_text(&detail_focused);
        assert_eq!(detail_text.matches("› Mi Monitor").count(), 1);
        assert!(!detail_text.contains("❯ Mi Monitor"));
        assert_eq!(detail_text.matches('▸').count(), 0);

        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::List;
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
    fn test_unconfigured_daemon_monitor_never_shows_invented_limits() {
        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = crate::tui::model::DaemonConnection::Connected;
        model.active_tab = Tab::Monitors;
        model.monitor_pane_focus = crate::tui::model::MonitorPaneFocus::Detail;

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
