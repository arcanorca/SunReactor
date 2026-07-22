use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{load_from_path, parse_str, write_default_to, ConfigError, ConfigSource};

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
