use chrono::{DateTime, Utc};
use sunreactor::config::{MonitorConfig, SolarPolicyConfig};
use sunreactor::policy::{self, PolicyContext};
use sunreactor::solar::Location;

fn main() {
    let config = SolarPolicyConfig {
        twilight_elevation_start: -6.0,
        day_elevation_full: 3.0,
        use_adaptive_zenith: true,
        max_step_pct_per_tick: 6,
        min_write_delta_pct: 2,
    };
    let monitors = vec![MonitorConfig {
        logical_id: "test".to_string(),
        min_pct: 6,
        max_pct: 38,
        gain: 1.0,
        transition_gamma: 1.4,
        milestone_adjustments: vec![],
        backend: sunreactor::backends::BackendKind::Backlight,
        enabled: true,
        selector: sunreactor::config::MonitorSelector::default(),
    }];
    let loc = Location::from_timezone_name(40.8, 29.2, "Europe/Istanbul").unwrap();
    let now = DateTime::parse_from_rfc3339("2026-06-21T19:01:39+03:00")
        .unwrap()
        .with_timezone(&Utc);

    let input = PolicyContext {
        now_utc: now,
        location: &loc,
        config: &config,
        weather_multiplier: None,
        monitors: &monitors,
    };
    let result = policy::compute_policy(&input).unwrap();
    println!("{:#?}", result);
}
