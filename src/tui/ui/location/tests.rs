use std::time::{Duration, Instant};

use ratatui::{backend::TestBackend, Terminal};

use crate::tui::{test_support::find_in_buffer, ui, InputMode, Model, Tab};

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
        crate::config::ThemeId::new("nord"),
        crate::config::ThemeId::new("hackergreen"),
    ] {
        let backend = TestBackend::new(85, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut model = Model::new();
        model.config.tui.theme = theme.clone();
        model.active_tab = Tab::Location;
        model.active_setting = 1; // Latitude focused

        let res = terminal.draw(|f| ui::ui(f, &mut model));
        assert!(res.is_ok());

        let buffer = terminal.backend().buffer().clone();
        let marker = find_in_buffer(&buffer, "❯").expect("focus marker present");
        assert_eq!(marker.2.fg, theme.palette().accent);
    }
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
    model.motion = crate::tui::motion::UiMotionState::new_at(now);

    // 1. Static state before acquisition
    terminal.draw(|f| ui::ui(f, &mut model)).unwrap();
    let buffer_static = terminal.backend().buffer().clone();
    assert!(find_in_buffer(&buffer_static, "Istanbul, TR").is_some());

    // 2. Acquisition ping triggered upon accepting a new location
    model.motion.trigger(
        crate::tui::motion::TransientKind::LocationAcquisition {
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
        crate::tui::motion::TransientKind::LocationAcquisition {
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
    crate::tui::update::update(&mut model, crate::tui::update::Message::Resize(140, 45));

    // Assert location data is never mutated by a resize
    assert_eq!(model.config.location.city, "Sydney, AU");
    assert!((model.config.location.latitude - (-33.8688)).abs() < 1e-6);
    assert!((model.config.location.longitude - 151.2093).abs() < 1e-6);
    assert_eq!(model.config.location.timezone, "Australia/Sydney");
    assert!(model.cell_metrics.cell_aspect > 0.0);
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
