    use crate::tui::command::{
        commands_for_footer, commands_for_workspace, CommandId, UiCommandContext,
    };
    use crate::tui::motion::{TransientKind, UiMotionState};
    use chrono::{DateTime, FixedOffset, TimeZone};
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::time::{Duration, Instant};
    use tui_input::Input;

    use crate::backends::BackendKind;
    use crate::policy::{AutomationMilestone, MonitorMilestone, MonitorMilestoneSchedule};
    use crate::tui::actions;
    use crate::tui::app::ModelEnvironment;
    use crate::tui::form::FormState;
    use crate::tui::model::{AutomationRegionFocus, DaemonConnection, Tab};
    use crate::tui::test_support::{
        buffer_row, buffer_text, configure_monitor_fixture, dummy_status, find_in_buffer,
        named_monitor_config, press, two_monitor_model,
    };
    use crate::tui::update;
    use crate::tui::{ui, Model};
    #[test]
    fn test_advanced_milestones_table_structure_comfortable() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.automation_focus = AutomationRegionFocus::Milestones;

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
        model.daemon_connection = DaemonConnection::Connected;
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
        model.automation_focus = AutomationRegionFocus::Milestones;
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
        actions::adjust_selected_monitor_milestone(&mut model, 1);
        let sched = model.selected_monitor_schedule().unwrap();
        assert_eq!(sched.milestones[0].minutes_offset, 1);
        assert_eq!(sched.milestones[0].base_time_local, make_time(6, 11)); // Base unchanged
        assert_eq!(sched.milestones[0].target_percent, 7); // Target unchanged

        // 2. Reset with r
        actions::reset_selected_monitor_milestone(&mut model);
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
        model.config.tui.theme = crate::config::ThemeId::new("nord");
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());
    }

    #[test]
    fn test_automation_instrument_smoke_across_all_themes() {
        use ratatui::style::Color;

        for theme in crate::config::ThemeId::all() {
            let mut model = Model::with_environment(ModelEnvironment::isolated());
            configure_monitor_fixture(
                &mut model,
                vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
            );
            model.config.location.city = String::from("Istanbul, TR");
            model.config.location.latitude = 41.0082;
            model.config.location.longitude = 28.9784;
            model.config.location.timezone = String::from("Europe/Istanbul");
            model.config.tui.theme = theme.clone();
            model.status = Some(dummy_status(1));
            model.daemon_connection = DaemonConnection::Connected;
            model.active_tab = Tab::Limits;
            model.automation_focus = AutomationRegionFocus::Curve;
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
        let mut model = Model::with_environment(ModelEnvironment::isolated());
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
        );
        model.config.location.city = String::from("Istanbul, TR");
        model.config.location.latitude = 41.0082;
        model.config.location.longitude = 28.9784;
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
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
        let mut model = Model::with_environment(ModelEnvironment::isolated());
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
        );
        let mut status = dummy_status(1);
        status.monitors[0].last_applied_percent = Some(88);
        model.status = Some(status);
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.motion.level = crate::config::MotionLevel::Off;
        model.refresh_monitor_milestones();

        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
        let known = buffer_text(terminal.backend().buffer());
        // 88 in the three-row digit font: `╭─╮ ╭─╮` over `├─┤ ├─┤` over `╰─╯ ╰─╯`.
        assert!(known.contains("Output"), "{known}");
        assert!(known.contains("├─┤ ├─┤"), "{known}");
        assert!(!known.contains("not written yet"), "{known}");

        model.status.as_mut().expect("fixture status").monitors[0].last_applied_percent = None;
        terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
        let unknown = buffer_text(terminal.backend().buffer());
        assert!(unknown.contains("Target · not written yet"), "{unknown}");
        assert!(!unknown.contains("├─┤ ├─┤"), "{unknown}");
    }

    #[test]
    fn test_render_target_instrument_pixel_glyph_for_pixel_themes() {
        let mut model = Model::with_environment(ModelEnvironment::isolated());
        configure_monitor_fixture(
            &mut model,
            vec![named_monitor_config("mon-0", "Mi Monitor", 5, 60)],
        );
        model.config.tui.theme = crate::config::ThemeId::new("amiga");
        let mut status = dummy_status(1);
        status.monitors[0].last_applied_percent = Some(44);
        model.status = Some(status);
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.motion.level = crate::config::MotionLevel::Off;
        model.refresh_monitor_milestones();

        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        terminal.draw(|frame| ui::ui(frame, &mut model)).unwrap();
        let known = buffer_text(terminal.backend().buffer());
        // 44 in the weather pixel font: `█  █ █  █` over `▀▀▀█ ▀▀▀█`.
        assert!(known.contains("Output"), "{known}");
        assert!(known.contains("▀▀▀█ ▀▀▀█"), "{known}");
        assert!(!known.contains("not written yet"), "{known}");
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
        model.daemon_connection = DaemonConnection::Connected;
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
    fn test_phase9_4_automation_monitor_switching_cycle() {
        let mut model = Model::new();
        model.status = Some(dummy_status(2));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.selected_monitor = 0;

        // Press ']' to switch to next monitor
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char(']'),
            )),
        );
        assert_eq!(model.selected_monitor, 1);

        // Press '[' to switch back
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Char('['),
            )),
        );
        assert_eq!(model.selected_monitor, 0);

        // Context sharing: switch to Tab 1 retains selected_monitor
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
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
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.automation_focus = AutomationRegionFocus::Milestones;
        model.selected_monitor_milestone = 0;

        // Up from milestone 0 moves focus to Curve shape
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Up,
            )),
        );
        assert_eq!(model.automation_focus, AutomationRegionFocus::Curve);

        // Right on curve steps gamma by +0.05
        let prev_gamma = model.config.monitors[0].transition_gamma;
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Right,
            )),
        );
        let new_gamma = model.config.monitors[0].transition_gamma;
        assert!((new_gamma - (prev_gamma + 0.05)).abs() < 1e-4);

        // Down moves back to Milestones
        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Down,
            )),
        );
        assert_eq!(model.automation_focus, AutomationRegionFocus::Milestones);
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
        model.form = FormState::new(&model.config);
        model.status = Some(dummy_status(2));
        model.daemon_connection = DaemonConnection::Connected;
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

        update::update(
            &mut model,
            update::Message::Key(crossterm::event::KeyEvent::from(
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
    fn test_automation_monitor_chips_switch_with_left_right() {
        let mut model = two_monitor_model();
        model.switch_to_tab(Tab::Limits);
        assert_eq!(
            model.automation_focus,
            AutomationRegionFocus::Selector,
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
        assert_eq!(model.automation_focus, AutomationRegionFocus::Curve);
        press(&mut model, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(model.automation_focus, AutomationRegionFocus::Selector);
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
    fn test_next_automation_event_calculations() {
        let mut model = Model::new();
        let offset = FixedOffset::east_opt(3 * 3600).unwrap();
        let make_time = |hour: u32, minute: u32| -> DateTime<FixedOffset> {
            offset
                .with_ymd_and_hms(2026, 9, 10, hour, minute, 0)
                .single()
                .unwrap()
        };

        model.monitor_milestones = vec![MonitorMilestoneSchedule {
            logical_id: String::from("mon-0"),
            milestones: vec![
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
                    adjusted_time_local: make_time(7, 15),
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
            ],
        }];

        let next_dawn = model
            .next_automation_milestone(&make_time(4, 0))
            .expect("next milestone found");
        assert_eq!(next_dawn.milestone_label, "Rise Start");
        assert_eq!(next_dawn.time_str, "06:00");
        assert_eq!(next_dawn.target_percent, 7);
        assert!(!next_dawn.is_tomorrow);

        let next_sunrise = model
            .next_automation_milestone(&make_time(6, 30))
            .expect("next milestone found");
        assert_eq!(next_sunrise.milestone_label, "Rise 25%");
        assert_eq!(next_sunrise.time_str, "07:15");
        assert_eq!(next_sunrise.minutes_offset, 15);
        assert_eq!(next_sunrise.target_percent, 25);
        assert!(!next_sunrise.is_tomorrow);

        let next_peak = model
            .next_automation_milestone(&make_time(7, 15))
            .expect("next milestone found");
        assert_eq!(next_peak.milestone_label, "Peak");
        assert_eq!(next_peak.time_str, "12:00");
        assert_eq!(next_peak.target_percent, 60);

        let next_tomorrow = model
            .next_automation_milestone(&make_time(22, 0))
            .expect("next milestone found");
        assert_eq!(next_tomorrow.milestone_label, "Rise Start");
        assert_eq!(next_tomorrow.time_str, "06:00");
        assert!(next_tomorrow.is_tomorrow);
    }

    #[test]
    fn test_phase5_6_automation_milestone_micro_settle() {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let now = Instant::now();
        let mut model = Model::new();
        model.status = Some(dummy_status(1));
        model.daemon_connection = DaemonConnection::Connected;
        model.active_tab = Tab::Limits;
        model.motion = UiMotionState::new_at(now);

        // 1. Milestone adjustment triggers micro-settle for milestone index 0
        model.motion.trigger(
            TransientKind::MilestoneAdjust { index: 0 },
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

    #[test]
    fn test_phase9_4_curve_upper_bound_uses_config_authority() {
        let mut model = Model::new();
        model.config.monitors = vec![crate::config::MonitorConfig {
            logical_id: String::from("mon-0"),
            transition_gamma: crate::config::MAX_TRANSITION_GAMMA - 0.05,
            ..Default::default()
        }];
        model.form = FormState::new(&model.config);
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

        model.form.monitor_curve_inputs[0] = Input::default().with_value(String::from("4.01"));
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
        let location =
            crate::solar::Location::from_timezone_name(41.0082, 28.9784, "Europe/Istanbul")
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
            (output.targets[0].effective_daylight_factor
                - output.targets[1].effective_daylight_factor)
                .abs()
                > f64::EPSILON
        );
    }

    #[test]
    fn test_phase9_4_curve_focus_footer_and_help_are_truthful() {
        let mut model = Model::new();
        model.active_tab = Tab::Limits;
        model.automation_focus = AutomationRegionFocus::Curve;

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
        model.form = FormState::new(&model.config);
        model.status = Some(dummy_status(1));
        model.active_tab = Tab::Limits;
        model.automation_focus = AutomationRegionFocus::Curve;

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

        model.automation_focus = AutomationRegionFocus::Milestones;
        let mut milestone_terminal = Terminal::new(TestBackend::new(85, 26)).unwrap();
        milestone_terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
        let milestone_buffer = milestone_terminal.backend().buffer().clone();
        assert!(find_in_buffer(&milestone_buffer, "±1 min").is_some());
        assert!(find_in_buffer(&milestone_buffer, "r Reset").is_some());
    }
