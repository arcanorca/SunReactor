use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{load_from_path, parse_str, render, write_default_to, ConfigError, ConfigSource};

const VALID_CONFIG: &str = r#"
[daemon]
tick_seconds = 60
dry_run = false
desktop_idle_sync = true
log_level = "info"

[location]
latitude = 41.0082
longitude = 28.9784
timezone = "UTC"

[solar_policy]
twilight_elevation_start = -6.0
day_elevation_full = 20.0

max_step_pct_per_tick = 6
min_write_delta_pct = 2

[[monitors]]
logical_id = "desk"
backend = "ddc"
enabled = true
min_pct = 20
max_pct = 90
gain = 1.0
connector = "DP-1"
ddc_bus = 6
ddc_address = 55

[weather]
enabled = false
provider = "openweather"
api_key_env = "OPENWEATHER_API_KEY"
refresh_minutes = 30
min_multiplier = 0.75
"#;

#[test]
fn parses_valid_config() {
    let config = parse_str(VALID_CONFIG, Path::new("valid.toml")).expect("config should parse");
    assert_eq!(config.monitors.len(), 1);
    assert_eq!(config.monitors[0].logical_id, "desk");
    assert!(!config.daemon.smooth_transition);
}

#[test]
fn parses_smooth_transition_opt_in() {
    let raw = VALID_CONFIG.replace(
        "dry_run = false",
        "dry_run = false\nsmooth_transition = true",
    );
    let config = parse_str(&raw, Path::new("smooth-transition.toml")).expect("config should parse");

    assert!(config.daemon.smooth_transition);
}

#[test]
fn external_change_hold_defaults_and_accepts_legacy_zero() {
    let config = parse_str(VALID_CONFIG, Path::new("external-hold-default.toml"))
        .expect("config should parse");
    assert_eq!(config.daemon.external_change_hold_minutes, 30);

    let raw = VALID_CONFIG.replace(
        "dry_run = false",
        "dry_run = false\nexternal_change_hold_minutes = 0",
    );
    let config = parse_str(&raw, Path::new("external-hold-zero.toml"))
        .expect("zero should preserve legacy behaviour");
    assert_eq!(config.daemon.external_change_hold_minutes, 0);
}

#[test]
fn tui_refresh_rate_is_persisted() {
    let raw = format!("{VALID_CONFIG}\n[tui]\nfps = 8\n");
    let config =
        parse_str(&raw, Path::new("tui-refresh-rate.toml")).expect("TUI refresh rate should parse");
    assert_eq!(config.tui.fps, 8);

    let rendered = render(&config).expect("config should serialize");
    let round_tripped = parse_str(&rendered, Path::new("round-trip-tui-fps.toml"))
        .expect("serialized TUI refresh rate should parse");
    assert_eq!(round_tripped.tui.fps, 8);
}

#[test]
fn topology_retargeting_defaults_to_false_and_round_trips_true() {
    let without_field =
        parse_str(VALID_CONFIG, Path::new("missing-field.toml")).expect("old config should parse");
    assert!(!without_field.monitors[0].allow_topology_retargeting);

    let with_field = VALID_CONFIG.replace(
        "enabled = true\n",
        "enabled = true\nallow_topology_retargeting = true\n",
    );
    let parsed = parse_str(&with_field, Path::new("explicit-field.toml"))
        .expect("explicit opt-in should parse");
    assert!(parsed.monitors[0].allow_topology_retargeting);
    let encoded = toml::to_string(&parsed).expect("config should serialize");
    let round_tripped =
        parse_str(&encoded, Path::new("round-trip.toml")).expect("serialized config should parse");
    assert!(round_tripped.monitors[0].allow_topology_retargeting);
}

#[test]
fn example_template_is_valid() {
    parse_str(
        super::DEFAULT_CONFIG_TEMPLATE,
        Path::new("examples/config.toml"),
    )
    .expect("example config should parse");
}

#[test]
fn rejects_duplicate_monitor_ids() {
    let raw = format!("{VALID_CONFIG}\n[[monitors]]\nlogical_id = \"desk\"\nbackend = \"backlight\"\nenabled = false\nmin_pct = 0\nmax_pct = 100\ngain = 1.0\nsysfs_path = \"/sys/class/backlight/intel_backlight\"\n");
    let error = parse_str(&raw, Path::new("duplicate.toml")).expect_err("config should fail");
    assert!(matches!(error, ConfigError::Validation(_)));
    assert!(error.to_string().contains("duplicate logical id"));
}

#[test]
fn rejects_semantically_overlapping_ddc_selectors() {
    let base = VALID_CONFIG
        .replace("connector = \"DP-1\"", "model = \"U2720Q\"")
        .replace("ddc_bus = 6\n", "");
    let raw = format!("{base}\n[[monitors]]\nlogical_id = \"right\"\nbackend = \"ddc\"\nenabled = true\nmin_pct = 20\nmax_pct = 90\ngain = 1.0\nserial = \"ABC123\"\nmodel = \"U2720Q\"\n");
    let error =
        parse_str(&raw, Path::new("overlap-selector.toml")).expect_err("config should fail");
    assert!(error.to_string().contains("may address the same display"));
}

#[test]
fn rejects_invalid_timezone() {
    let raw = VALID_CONFIG.replace("timezone = \"UTC\"", "timezone = \"Mars/Olympus\"");
    let error = parse_str(&raw, Path::new("timezone.toml")).expect_err("config should fail");
    assert!(error.to_string().contains("location.timezone"));
}

#[test]
fn rejects_low_weather_refresh() {
    let raw = VALID_CONFIG.replace("refresh_minutes = 30", "refresh_minutes = 1");
    let error = parse_str(&raw, Path::new("weather.toml")).expect_err("config should fail");
    assert!(error.to_string().contains("weather.refresh_minutes"));
}

#[test]
fn cloud_dimming_helpers_round_trip_the_canonical_multiplier() {
    let mut weather = super::WeatherConfig::default();
    assert_eq!(weather.cloud_dimming_percent(), 25);
    weather
        .set_cloud_dimming_percent(50)
        .expect("50% should be accepted");
    assert!((weather.min_multiplier - 0.5).abs() < f64::EPSILON);
    assert_eq!(weather.cloud_dimming_percent(), 50);
    weather
        .set_cloud_dimming_percent(0)
        .expect("0% should be accepted");
    assert!((weather.min_multiplier - 1.0).abs() < f64::EPSILON);

    weather.min_multiplier = 0.3;
    assert_eq!(weather.cloud_dimming_percent(), 50);
}

#[test]
fn rejects_cloud_dimming_above_fifty_percent() {
    let mut weather = super::WeatherConfig::default();
    let error = weather
        .set_cloud_dimming_percent(51)
        .expect_err("51% must be rejected");
    assert!(error.contains("0..=50"));
}

#[test]
fn loads_legacy_low_multiplier_clamped_without_rewriting_file() {
    let raw = VALID_CONFIG.replace("min_multiplier = 0.75", "min_multiplier = 0.3");
    let path = std::env::temp_dir().join(format!(
        "sunreactor-config-normalize-{}.toml",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos()
    ));
    fs::write(&path, &raw).expect("fixture config should be writable");

    let report = load_from_path(&path).expect("legacy multiplier should load");
    assert!((report.config.weather.min_multiplier - 0.5).abs() < f64::EPSILON);
    assert_eq!(report.config.weather.cloud_dimming_percent(), 50);
    assert_eq!(
        fs::read_to_string(&path).expect("fixture should remain readable"),
        raw
    );
    fs::remove_file(path).expect("fixture should be removed");
}

#[test]
fn rejects_zero_and_above_one_min_multiplier() {
    for value in ["0.0", "1.2"] {
        let raw = VALID_CONFIG.replace(
            "min_multiplier = 0.75",
            &format!("min_multiplier = {value}"),
        );
        let error = parse_str(&raw, Path::new("invalid-multiplier.toml"))
            .expect_err("invalid multiplier should fail");
        assert!(error.to_string().contains("weather.min_multiplier"));
    }
}

#[test]
fn rejects_removed_legacy_global_min_max_fields() {
    let raw = VALID_CONFIG.replace(
        "[solar_policy]\n",
        "[solar_policy]\nnight_min_pct = 18\nday_max_pct = 90\n",
    );

    let error = parse_str(&raw, Path::new("legacy.toml"))
        .expect_err("removed legacy min/max fields should fail");
    assert!(error.to_string().contains("solar_policy.night_min_pct"));
    assert!(error.to_string().contains("solar_policy.day_max_pct"));
}

#[test]
fn rejects_tick_rate_that_busy_polls() {
    let raw = VALID_CONFIG.replace("tick_seconds = 60", "tick_seconds = 1");
    let error = parse_str(&raw, Path::new("daemon.toml")).expect_err("config should fail");
    assert!(error.to_string().contains("daemon.tick_seconds"));
}

#[test]
fn rejects_unbounded_operational_timeouts() {
    let mut config = super::Config::default();
    config.daemon.desktop_idle_timeout_minutes = 1_441;
    config.daemon.apply_reassert_minutes = 1_441;
    config.daemon.external_change_hold_minutes = 1_441;
    config.daemon.ddc_timeout_seconds = 61;
    config.daemon.backlight_timeout_seconds = 0;

    let error = config
        .validate()
        .expect_err("operational limits should fail");
    let text = error.to_string();
    assert!(text.contains("daemon.desktop_idle_timeout_minutes"));
    assert!(text.contains("daemon.apply_reassert_minutes"));
    assert!(text.contains("daemon.external_change_hold_minutes"));
    assert!(text.contains("daemon.ddc_timeout_seconds"));
    assert!(text.contains("daemon.backlight_timeout_seconds"));
}

#[test]
fn accepts_disabled_idle_timeout_and_bounded_backend_timeouts() {
    let mut config = super::Config::default();
    config.daemon.desktop_idle_timeout_minutes = 0;
    config.daemon.ddc_timeout_seconds = 1;
    config.daemon.backlight_timeout_seconds = 60;
    config
        .validate()
        .expect("bounded timeout values should pass");
}

#[test]
fn accepts_desktop_idle_sync_override() {
    let raw = VALID_CONFIG.replace("desktop_idle_sync = true", "desktop_idle_sync = false");
    let config = parse_str(&raw, Path::new("desktop-idle-sync.toml")).expect("config should parse");

    assert!(!config.daemon.desktop_idle_sync);
}

#[test]
fn load_returns_defaults_when_file_is_missing() {
    let path = Path::new("missing-config.toml");
    let report = load_from_path(path).expect("missing config should fall back to defaults");
    assert_eq!(report.source, ConfigSource::Defaults);
    assert_eq!(report.path, path);
}

#[test]
fn write_default_creates_config_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should work")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("sunreactor-config-test-{unique}"));
    let path = dir.join("config.toml");

    let written = write_default_to(&path).expect("should write default config");
    let contents = fs::read_to_string(&written).expect("written config should be readable");

    assert_eq!(written, path);
    assert!(contents.contains("[daemon]"));
    assert!(contents.contains("[solar_policy]"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&written)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "new config file should be user-private 0600");

        let parent_mode = fs::metadata(&dir)
            .expect("parent metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            parent_mode, 0o700,
            "new config directory should be user-private 0700"
        );
    }

    fs::remove_file(&written).ok();
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn rejects_duplicate_monitor_milestone_adjustments() {
    let raw = VALID_CONFIG.replace(
        "[weather]\n",
        "[[monitors.milestone_adjustments]]\nmilestone = \"rise_25\"\nminutes_offset = 5\n[[monitors.milestone_adjustments]]\nmilestone = \"rise_25\"\nminutes_offset = 10\n\n[weather]\n",
    );
    let error =
        parse_str(&raw, Path::new("duplicate-milestones.toml")).expect_err("config should fail");
    assert!(matches!(error, ConfigError::Validation(_)));
    assert!(error.to_string().contains("duplicate milestone `rise_25`"));
}

#[test]
fn tui_effects_defaults_and_round_trips() {
    use super::MotionLevel;

    // 1. Missing [tui] or missing effects defaults to Instrument
    let config = parse_str(VALID_CONFIG, Path::new("valid.toml")).expect("config should parse");
    assert_eq!(config.tui.effects, MotionLevel::Instrument);

    // 2. Explicit effects in [tui]
    let with_reduced = format!("{VALID_CONFIG}\n[tui]\neffects = \"reduced\"\n");
    let parsed_reduced = parse_str(&with_reduced, Path::new("reduced.toml")).expect("should parse");
    assert_eq!(parsed_reduced.tui.effects, MotionLevel::Reduced);

    let with_off = format!("{VALID_CONFIG}\n[tui]\neffects = \"off\"\n");
    let parsed_off = parse_str(&with_off, Path::new("off.toml")).expect("should parse");
    assert_eq!(parsed_off.tui.effects, MotionLevel::Off);

    // 3. Serialization writes lowercase human-readable string
    let encoded = toml::to_string(&parsed_reduced).expect("should serialize");
    assert!(encoded.contains("effects = \"reduced\""));

    let round_tripped = parse_str(&encoded, Path::new("roundtrip.toml")).expect("should parse");
    assert_eq!(round_tripped.tui.effects, MotionLevel::Reduced);

    // 4. Invalid effects value fails safely
    let with_invalid = format!("{VALID_CONFIG}\n[tui]\neffects = \"unknown_motion\"\n");
    assert!(parse_str(&with_invalid, Path::new("invalid.toml")).is_err());
}

fn config_invalid_in_every_section() -> super::Config {
    use super::{MonitorConfig, MonitorMilestoneAdjustment};
    use crate::backends::BackendKind;

    let mut config = super::Config::default();
    config.daemon.tick_seconds = 1;
    config.daemon.probe_seconds = 1;
    config.daemon.desktop_idle_timeout_minutes = 1_441;
    config.daemon.apply_reassert_minutes = 1_441;
    config.daemon.external_change_hold_minutes = 1_441;
    config.daemon.ddc_timeout_seconds = 0;
    config.daemon.backlight_timeout_seconds = 61;
    config.location.latitude = 91.0;
    config.location.longitude = -181.0;
    config.location.timezone = String::from("Nope/Zone");
    config.solar_policy.max_step_pct_per_tick = 0;
    config.solar_policy.min_write_delta_pct = 101;
    config.solar_policy.twilight_elevation_start = 30.0;
    config.solar_policy.day_elevation_full = 10.0;

    let mut broken = MonitorConfig {
        logical_id: String::from("  "),
        transition_gamma: f64::NAN,
        min_pct: 110,
        max_pct: 105,
        gain: 5.0,
        ..MonitorConfig::default()
    };
    broken.milestone_adjustments = vec![
        MonitorMilestoneAdjustment::default(),
        MonitorMilestoneAdjustment {
            minutes_offset: 800,
            ..MonitorMilestoneAdjustment::default()
        },
    ];
    let ddc = |id: &str, address: Option<u16>| {
        let mut monitor = MonitorConfig {
            logical_id: id.to_owned(),
            backend: BackendKind::Ddc,
            ..MonitorConfig::default()
        };
        monitor.selector.ddc_bus = Some(3);
        monitor.selector.ddc_address = address;
        monitor
    };
    config.monitors = vec![
        broken,
        ddc("dup", Some(200)),
        ddc("dup", None),
        ddc("other", None),
    ];
    config.weather.enabled = true;
    config.weather.provider = None;
    config.weather.api_key_env = Some(String::from("  "));
    config.weather.api_key = Some(String::new());
    config.weather.refresh_minutes = 1;
    config.weather.min_multiplier = f64::NAN;
    config
}

/// Characterization: pins the exact field, message and order of every
/// validation error when all sections are invalid at once.
#[test]
fn characterization_validation_error_order_and_messages() {
    let mut config = config_invalid_in_every_section();
    let Err(ConfigError::Validation(errors)) = config.validate() else {
        panic!("every section is invalid");
    };
    let lines = errors.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "daemon.tick_seconds: must be at least 5 to avoid busy polling",
            "daemon.probe_seconds: must be 0 (disabled) or within 2..=3600",
            "daemon.desktop_idle_timeout_minutes: must be 0 (disabled) or at most 1440 minutes",
            "daemon.apply_reassert_minutes: must be at most 1440 minutes",
            "daemon.external_change_hold_minutes: must be within 0..=1440 minutes",
            "daemon.ddc_timeout_seconds: must be within 1..=60 seconds",
            "daemon.backlight_timeout_seconds: must be within 1..=60 seconds",
            "location.latitude: must be within -90..=90",
            "location.longitude: must be within -180..=180",
            "location.timezone: invalid timezone `Nope/Zone` (must be a valid IANA zone or POSIX string)",
            "solar_policy.min_write_delta_pct: must be within 0..=100",
            "solar_policy: twilight_elevation_start must be lower than day_elevation_full",
            "solar_policy.max_step_pct_per_tick: must be greater than 0",
            "monitors[0].transition_gamma: must be finite and within 0 < gamma <= 4",
            "monitors[0].logical_id: must not be empty",
            "monitors[0].min_pct: must be within 0..=100",
            "monitors[0].max_pct: must be within 0..=100",
            "monitors[0].min_pct: must be less than or equal to max_pct",
            "monitors[0].gain: must be finite and within 0 < gain <= 4",
            "monitors[0].selector: enabled monitors must define at least one selector field",
            "monitors[0].milestone_adjustments[1].milestone: duplicate milestone `rise_start`",
            "monitors[0].milestone_adjustments[1].minutes_offset: must stay within -720..=720 minutes",
            "monitors[1].ddc_address: must fit within the 7-bit I2C address range",
            "monitors[1].selector: Ddc selector `ddc_address` is invalid: only the standard DDC/CI slave address 55 is supported",
            "monitors[2].logical_id: duplicate logical id `dup`",
            "monitors[3].selector: monitors[2] and monitors[3] use DDC selectors that may address the same display (Equal): `[\"--bus\", \"3\"]` overlaps `[\"--bus\", \"3\"]`; use pairwise disjoint selectors",
            "weather.refresh_minutes: must be at least 10 minutes to keep refresh bounded",
            "weather.min_multiplier: must be finite and within 0 < min_multiplier <= 1",
            "weather.provider: must be set when weather is enabled",
            "weather.api_key_env: must not be empty when set",
            "weather.api_key: must not be empty when set",
        ]
    );

    config.weather.api_key_env = None;
    config.weather.api_key = None;
    let Err(ConfigError::Validation(errors)) = config.validate() else {
        panic!("every section is invalid");
    };
    assert_eq!(
        errors.last().map(ToString::to_string).as_deref(),
        Some("weather: enabled weather requires api_key_env or api_key")
    );
}

/// Configs written before the operational bounds existed must still load:
/// out-of-range values are clamped with a report warning, and the file on
/// disk is left untouched.
#[test]
fn loads_legacy_out_of_range_operational_values_clamped_with_warnings() {
    let raw = VALID_CONFIG.replace(
        "tick_seconds = 60\n",
        "tick_seconds = 60\ndesktop_idle_timeout_minutes = 5000\napply_reassert_minutes = 2000\nddc_timeout_seconds = 0\nbacklight_timeout_seconds = 120\n",
    );
    let path = std::env::temp_dir().join(format!(
        "sunreactor-config-legacy-bounds-{}.toml",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos()
    ));
    fs::write(&path, &raw).expect("fixture config should be writable");

    let report = load_from_path(&path).expect("legacy operational values should load");
    let daemon = &report.config.daemon;
    assert_eq!(daemon.desktop_idle_timeout_minutes, 1_440);
    assert_eq!(daemon.apply_reassert_minutes, 1_440);
    assert_eq!(daemon.ddc_timeout_seconds, 1);
    assert_eq!(daemon.backlight_timeout_seconds, 60);
    for field in [
        "daemon.desktop_idle_timeout_minutes",
        "daemon.apply_reassert_minutes",
        "daemon.ddc_timeout_seconds",
        "daemon.backlight_timeout_seconds",
    ] {
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains(field)),
            "missing clamp warning for {field}: {:?}",
            report.warnings
        );
    }
    assert_eq!(
        fs::read_to_string(&path).expect("fixture should remain readable"),
        raw
    );
    fs::remove_file(path).expect("fixture should be removed");
}

/// Our own structured save path still rejects out-of-range values.
#[test]
fn save_rejects_out_of_range_operational_values() {
    let mut config = super::Config::default();
    config.daemon.ddc_timeout_seconds = 0;
    let path = std::env::temp_dir().join("sunreactor-config-never-written.toml");
    let error = super::save_to_path(&config, &path).expect_err("save must validate strictly");
    assert!(error.to_string().contains("daemon.ddc_timeout_seconds"));
    assert!(!path.exists());
}
