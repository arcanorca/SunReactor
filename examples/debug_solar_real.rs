use chrono::Utc;
use std::fs;
use sunreactor::config::Config;
use sunreactor::policy::{self, PolicyContext};
use sunreactor::solar::Location;

fn main() {
    let toml = fs::read_to_string("/home/arcanorca/.config/sunreactor/config.toml").unwrap();
    let config: Config = toml::from_str(&toml).unwrap();
    let now = Utc::now();
    let loc = Location::from_timezone_name(
        config.location.latitude,
        config.location.longitude,
        &config.location.timezone,
    )
    .unwrap();

    let input = PolicyContext {
        now_utc: now,
        location: &loc,
        config: &config.solar_policy,
        weather_multiplier: None,
        monitors: &config.monitors,
    };
    println!("Now UTC: {:?}", now);
    println!(
        "Now Local: {:?}",
        sunreactor::solar::local_datetime_at_utc(now, input.location).unwrap()
    );
    let result = policy::compute_policy(&input).unwrap();
    println!("{:#?}", result);
}
