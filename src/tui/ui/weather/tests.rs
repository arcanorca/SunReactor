    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    use super::WeatherResponsiveMode;
    use crate::ipc::{StatusResponse, WeatherStatus};
    use crate::state::ForecastPoint;
    use crate::tui::model::{DaemonConnection, Tab, TargetPreview};
    use crate::tui::test_support::{dummy_status, find_in_buffer};
    use crate::tui::ui;
    use crate::tui::update::{self, Message};
    use crate::tui::worker::IpcEvent;
    use crate::tui::Model;
    use crate::weather::{WeatherCondition, WeatherDayPhase, WeatherSourceKind, WeatherState};

    fn weather_model(state: WeatherState, condition: WeatherCondition) -> Model {
        let mut model = Model::new();
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;
        model.config.weather.enabled = true;
        model.config.location.city = String::from("Istanbul");
        model.config.location.timezone = String::from("Europe/Istanbul");
        model.status = Some(StatusResponse {
            daemon_alive: true,
            config_path: String::new(),
            tick_seconds: 60,
            dry_run: false,
            suspended: false,
            desktop_idle_dimmed: false,
            suspend_until_epoch_s: None,
            manual_override_active: false,
            per_monitor_override_until_epoch_s: None,
            global_override_percent: None,
            global_override_until_epoch_s: None,
            configured_monitors: 0,
            stateful_monitors: 0,
            weather: Some(WeatherStatus {
                enabled: true,
                state,
                active: matches!(state, WeatherState::Ready),
                stale: state == WeatherState::Stale,
                provider: Some(String::from("OpenWeather")),
                fetched_at_epoch_s: Some(1_700_000_000),
                valid_at_epoch_s: Some(1_700_000_000),
                source_kind: Some(WeatherSourceKind::Forecast),
                last_refresh_attempt_epoch_s: Some(1_700_000_000),
                next_refresh_at_epoch_s: Some(1_700_001_800),
                consecutive_failures: 0,
                last_error: None,
                cloud_cover_percent: Some(match condition {
                    WeatherCondition::Clear => 0,
                    WeatherCondition::PartlyCloudy => 42,
                    WeatherCondition::Cloudy => 80,
                    _ => 90,
                }),
                temperature: Some(23.0),
                condition,
                condition_description: Some(String::from("fixture conditions")),
                day_phase: Some(WeatherDayPhase::Day),
                forecast: vec![
                    ForecastPoint {
                        dt_epoch_s: 1_700_003_600,
                        cloud_cover_percent: 20,
                        temperature: 22.0,
                        ..Default::default()
                    },
                    ForecastPoint {
                        dt_epoch_s: 1_700_007_200,
                        cloud_cover_percent: 45,
                        temperature: 20.0,
                        ..Default::default()
                    },
                    ForecastPoint {
                        dt_epoch_s: 1_700_010_800,
                        cloud_cover_percent: 80,
                        temperature: 19.0,
                        ..Default::default()
                    },
                ],
                multiplier: (state != WeatherState::Stale).then_some(0.8),
                ..Default::default()
            }),
            monitors: Vec::new(),
            solar_elevation: Some(20.0),
            now_epoch_s: 1_700_000_120,
            sunrise_epoch_s: None,
            sunset_epoch_s: None,
            lunar_phase: None,
            theme: None,
            theme_warning: None,
            temperature_unit: None,
            location_city: None,
        });
        model
    }

    fn render(model: &mut Model, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height))
            .expect("test terminal should initialize");
        terminal
            .draw(|frame| crate::tui::ui::ui(frame, model))
            .expect("weather screen should render");
        buffer_text(terminal.backend().buffer())
    }

    fn buffer_text(buffer: &Buffer) -> String {
        let mut screen = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                screen.push_str(buffer.get(x, y).symbol());
            }
            screen.push('\n');
        }
        screen
    }

    #[test]
    fn weather_art_survives_the_supported_terminal_sizes() {
        for (width, height) in [(80, 24), (100, 30), (120, 35), (140, 40), (160, 45)] {
            let mut model = weather_model(WeatherState::Ready, WeatherCondition::Clear);
            let screen = render(&mut model, width, height);

            assert!(
                screen.chars().any(|c| matches!(c, '▀' | '▄' | '█')),
                "{width}x{height}: {screen}"
            );
            assert!(screen.contains("°C"), "{width}x{height}: {screen}");
            assert!(screen.contains("Clear"), "{width}x{height}: {screen}");
        }
    }

    #[test]
    fn visually_distinct_conditions_use_their_own_block_art() {
        let art_region = |condition| {
            let mut model = weather_model(WeatherState::Ready, condition);
            let screen = render(&mut model, 120, 35);
            let first = screen
                .lines()
                .position(|line| line.contains("╭ Current conditions"))
                .expect("conditions panel");
            screen
                .lines()
                .skip(first + 1)
                .take(7)
                .map(|line| line.chars().take(40).collect::<String>())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let cloudy = art_region(WeatherCondition::Cloudy);
        let rain = art_region(WeatherCondition::Rain);
        let snow = art_region(WeatherCondition::Snow);
        assert_ne!(cloudy, rain);
        assert_ne!(rain, snow);
        assert_ne!(cloudy, snow);
    }

    #[test]
    fn night_shows_the_real_moon_phase() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Clear);
        let weather = model
            .status
            .as_mut()
            .and_then(|status| status.weather.as_mut())
            .expect("weather fixture should exist");
        weather.day_phase = Some(WeatherDayPhase::Night);
        // 2023-11-14 22:15 UTC: two days after the new moon of 13 November.
        let screen = render(&mut model, 120, 35);
        assert!(screen.contains("Waxing crescent"), "{screen}");
        assert!(!screen.contains("Full moon"), "{screen}");
    }

    #[test]
    fn tiny_weather_keeps_the_input_and_drops_the_art() {
        let mut tiny = weather_model(WeatherState::Ready, WeatherCondition::Clear);
        let tiny_screen = render(&mut tiny, 45, 12);
        assert!(tiny_screen.contains("23.0°C"));
        assert!(tiny_screen.contains("Clear"));
        assert!(!tiny_screen
            .chars()
            .any(|c| ('\u{2801}'..='\u{28FF}').contains(&c)));
    }

    #[test]
    fn loading_and_failures_do_not_claim_unconfirmed_conditions() {
        let mut loading = weather_model(WeatherState::Loading, WeatherCondition::Unknown);
        loading
            .status
            .as_mut()
            .and_then(|status| status.weather.as_mut())
            .expect("weather fixture should exist")
            .cloud_cover_percent = None;
        let loading_screen = render(&mut loading, 100, 30);
        assert!(loading_screen.contains("Loading weather"));
        assert!(!loading_screen
            .chars()
            .any(|c| ('\u{2801}'..='\u{28FF}').contains(&c)));

        let mut provider_failure =
            weather_model(WeatherState::ProviderError, WeatherCondition::Clear);
        let failure_screen = render(&mut provider_failure, 100, 30);
        assert!(failure_screen.contains("Weather unavailable"));
        assert!(failure_screen.contains("r Retry"), "{failure_screen}");
        assert!(!failure_screen
            .chars()
            .any(|c| ('\u{2801}'..='\u{28FF}').contains(&c)));
    }

    #[test]
    fn stale_weather_keeps_art_but_is_not_presented_as_affecting_brightness() {
        let mut model = weather_model(WeatherState::Stale, WeatherCondition::Rain);
        let screen = render(&mut model, 120, 35);

        assert!(screen.chars().any(|c| matches!(c, '▀' | '▄' | '█')));
        assert!(screen.contains("Stale"), "{screen}");
        assert!(!screen.contains("Dims daylight"), "{screen}");
        assert!(screen.contains("r Retry"), "{screen}");
    }

    #[test]
    fn fresh_weather_is_not_called_current_and_offers_refresh_not_retry() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Cloudy);
        let screen = render(&mut model, 120, 35);
        assert!(!screen.contains("Current weather"), "{screen}");
        assert!(!screen.contains("CURRENT SKY"), "{screen}");
        assert!(screen.contains("Current conditions"), "{screen}");
        assert!(screen.contains("r Refresh"), "{screen}");
        assert!(!screen.contains("Retry"), "{screen}");
    }

    #[test]
    fn weather_layout_breakpoints_fit_their_required_content() {
        use ratatui::layout::Rect;
        assert_eq!(
            WeatherResponsiveMode::from_rect(Rect::new(0, 0, 96, 18)),
            WeatherResponsiveMode::Large
        );
        assert_eq!(
            WeatherResponsiveMode::from_rect(Rect::new(0, 0, 72, 14)),
            WeatherResponsiveMode::Medium
        );
        assert_eq!(
            WeatherResponsiveMode::from_rect(Rect::new(0, 0, 47, 9)),
            WeatherResponsiveMode::Tiny
        );
    }

    #[test]
    fn forecast_pairs_the_temperature_chart_with_per_hour_icons() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Cloudy);
        let screen = render(&mut model, 160, 45);
        println!("RENDER 160x45:\n{screen}");
        assert!(screen.contains("24 hour forecast"), "{screen}");
        // Every sample's temperature labels the chart once.
        for label in ["22°", "20°", "19°"] {
            assert!(screen.contains(label), "{label}: {screen}");
        }
        assert!(!screen.contains("cloud cover"), "{screen}");
        assert!(screen.chars().any(|c| matches!(c, '▁'..='█')), "{screen}");
    }

    #[test]
    fn weather_refresh_hints_follow_real_dispatch_and_state() {
        let ready = weather_model(WeatherState::Ready, WeatherCondition::Clear);
        assert_eq!(
            crate::tui::ui::weather_model::weather_action_label(&ready),
            Some("Refresh")
        );
        let ready_commands = crate::tui::command::commands_for_footer(&ready);
        assert_eq!(ready_commands.current_commands[0].keys, "r");

        let failed = weather_model(WeatherState::ProviderError, WeatherCondition::Unknown);
        assert_eq!(
            crate::tui::ui::weather_model::weather_action_label(&failed),
            Some("Retry")
        );

        let loading = weather_model(WeatherState::Loading, WeatherCondition::Unknown);
        assert_eq!(
            crate::tui::ui::weather_model::weather_action_label(&loading),
            None
        );
        assert!(crate::tui::command::commands_for_footer(&loading)
            .current_commands
            .is_empty());
    }

    #[test]
    fn weather_stays_about_the_weather() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Cloudy);
        model.monitor_targets_now = vec![TargetPreview {
            logical_id: String::from("mon-0"),
            solar_percent: 80,
            weather_percent: 64,
        }];
        let screen = render(&mut model, 160, 45);
        assert!(!screen.contains("Brightness effect"), "{screen}");
        assert!(!screen.contains("Dims daylight"), "{screen}");
        assert!(screen.contains("SunReactor impact"), "{screen}");
        let cloud_row = screen
            .lines()
            .find(|line| line.contains("Cloud cover"))
            .unwrap_or_default();
        assert!(cloud_row.contains("80%"), "{screen}");
    }
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
            model.daemon_connection = DaemonConnection::Connected;
            model.status = Some(dummy_status(1));

            terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            assert!(find_in_buffer(&buffer, "╭ Weather").is_some());
            assert!(find_in_buffer(&buffer, "Weather is off").is_some());
            assert!(find_in_buffer(&buffer, "Enable weather in Settings").is_some());
            assert!(
                find_in_buffer(&buffer, "Weather integration is disabled in configuration")
                    .is_none()
            );
            assert!(find_in_buffer(&buffer, "ATMOSPHERIC SUBSYSTEM").is_none());
            assert!(find_in_buffer(&buffer, "OpenWeather API Key").is_none());
        }

        // 2. Unavailable: enabled, but no weather data returned yet
        {
            let mut model = Model::new();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            model.daemon_connection = DaemonConnection::Connected;
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
            .start_activity(crate::tui::motion::ActiveActivity::WeatherRefresh { started_at: now });
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
        let styles = crate::tui::theme::SemanticStyles::from_palette(
            &crate::config::ThemeId::new("amber").palette(),
        );
        let style_col0 = crate::tui::motion::shimmer_style_for_column(
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
            crate::tui::motion::TransientKind::WeatherUpdate,
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
        model.daemon_connection = DaemonConnection::Connected;
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
            crate::config::ThemeId::new("amber"),
            crate::config::ThemeId::new("nord"),
            crate::config::ThemeId::new("hackergreen"),
            crate::config::ThemeId::new("grayscale"),
            crate::config::ThemeId::new("commodore64"),
        ] {
            let backend = TestBackend::new(120, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut model = Model::new();
            model.config.tui.theme = theme.clone();
            model.config.weather.enabled = true;
            model.active_tab = Tab::Weather;
            model.daemon_connection = DaemonConnection::Connected;
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

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::Fresh
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

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::Fresh
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

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::RefreshDue
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

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::RefreshDue
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

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::RefreshDue
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

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::Stale
            );
            assert_eq!(telem.freshness_label, "▲ Stale");
        }
    }

    #[test]
    fn test_phase8_1_policy_provenance_and_stale_behavior() {
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;

        // 1. Stale cache: multiplier is None, excluded from automation, solar curve unattenuated
        {
            let mut status = dummy_status(1);
            status.solar_elevation = Some(45.0); // Daylight
            status.now_epoch_s = 1_700_005_000;
            let mut ws = dummy_weather_status(true, false, true, Some(60), None, 8);
            ws.fetched_at_epoch_s = Some(1_700_000_000);
            status.weather = Some(ws);
            model.status = Some(status);

            let policy = crate::tui::ui::weather_model::compute_atmospheric_policy(&model);
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

            let telem = crate::tui::ui::weather_model::atmospheric_telemetry(&model);
            assert_eq!(
                telem.freshness,
                crate::tui::ui::weather_model::FreshnessState::RefreshFailedWithCache
            );
            assert_eq!(telem.policy.multiplier, Some(0.75));
            assert!(telem.policy.delta_percent.is_some());
        }

        // 3. Disabled weather: multiplier is None, explanation notes disabled state
        {
            model.config.weather.enabled = false;
            let policy = crate::tui::ui::weather_model::compute_atmospheric_policy(&model);
            assert_eq!(policy.multiplier, None);
            assert!(policy.explanation.contains("Weather disabled"));
        }
    }

    #[test]
    fn test_phase8_1_forecast_provenance_and_timezone_sampling() {
        // 1. Timezone and time-format correctness
        // 1700000000 = 2023-11-14 22:13:20 UTC
        // In Europe/Istanbul (UTC+3), this is 2023-11-15 01:13:20
        let label_24h = crate::tui::ui::weather_model::forecast_time_label(
            1_700_000_000,
            false,
            "Europe/Istanbul",
        );
        assert_eq!(label_24h, "01:13");

        let label_12h = crate::tui::ui::weather_model::forecast_time_label(
            1_700_000_000,
            true,
            "Europe/Istanbul",
        );
        assert_eq!(label_12h, "01:13 AM");

        // In America/New_York (UTC-5 in Nov, EST), 22:13 UTC is 17:13 (5:13 PM)
        let label_ny = crate::tui::ui::weather_model::forecast_time_label(
            1_700_000_000,
            false,
            "America/New_York",
        );
        assert_eq!(label_ny, "17:13");

        // 2. Downsampling behavior: in narrow viewport (< 55 cols for 8 points), table downsamples by step=2
        let mut model = Model::new();
        model.config.weather.enabled = true;
        model.active_tab = Tab::Weather;
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;
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
        model.daemon_connection = DaemonConnection::Connected;

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
        model.daemon_connection = DaemonConnection::Connected;

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
        model.daemon_connection = DaemonConnection::Connected;

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
    fn weather_pixel_font_and_forecast_strip_layout() {
        let mut model = weather_model(WeatherState::Ready, WeatherCondition::Clear);
        let screen = render(&mut model, 160, 45);

        // Current conditions render with the 3-row pixel art block font
        assert!(screen.contains("°C"), "{screen}");
        assert!(
            screen.contains("▄") || screen.contains("▀") || screen.contains("█"),
            "Pixel art font glyphs expected in screen: {screen}"
        );

        // Forecast strip legend is clean and shows only Temperature
        assert!(screen.contains("Temperature"), "{screen}");
        assert!(!screen.contains("Precipitation chance"), "{screen}");

        // Forecast samples display temperatures with degree symbols
        assert!(screen.contains("22°"), "{screen}");
    }
