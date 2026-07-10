use crate::config::WeatherConfig;
use crate::solar::Location;
use crate::state::WeatherSnapshotMetadata;

pub mod client;
pub mod engine;
pub mod openweather;
pub mod smoothing;
pub mod types;

pub(crate) use client::*;
pub use engine::*;
pub use openweather::*;
pub use smoothing::*;
pub use types::*;

pub fn resolve_modifier_with_provider<P, E>(
    config: &WeatherConfig,
    location: &Location,
    cached: Option<&WeatherSnapshotMetadata>,
    now_epoch_s: u64,
    next_refresh_at_epoch_s: Option<u64>,
    force_refresh: bool,
    consecutive_refresh_failures: u32,
    provider: &P,
    environment: &E,
) -> WeatherResolution
where
    P: WeatherProvider,
    E: EnvironmentReader,
{
    if !config.enabled {
        return WeatherResolution {
            modifier: None,
            snapshot: None,
            next_refresh_at_epoch_s: None,
            error: None,
            refresh_attempted: false,
        };
    }

    let mut snapshot = cached.cloned();
    let mut error = None;

    let request = match provider_request(config, location, now_epoch_s, environment) {
        Ok(req) => req,
        Err(err) => {
            if matches!(err, WeatherError::MissingApiKey { .. }) {
                return WeatherResolution {
                    modifier: None,
                    snapshot: None,
                    next_refresh_at_epoch_s: None,
                    error: Some(err),
                    refresh_attempted: false,
                };
            }
            error = Some(err);
            // We can't build a request, but we might still use the cached snapshot
            // We'll proceed so the logic can use the old snapshot
            return WeatherResolution {
                modifier: snapshot.as_ref().and_then(|s| snapshot_modifier(config, s, now_epoch_s)),
                snapshot,
                next_refresh_at_epoch_s: next_refresh_at_epoch_s.or(Some(now_epoch_s + WEATHER_FAILURE_RETRY_MAX_SECONDS)),
                error,
                refresh_attempted: false,
            };
        }
    };

    let refresh_due = force_refresh
        || next_refresh_at_epoch_s
            .is_none_or(|refresh_at_epoch_s| now_epoch_s >= refresh_at_epoch_s);

    if refresh_due {
        match provider.fetch_snapshot(&request) {
            Ok(fresh_snapshot) => {
                snapshot = Some(merge_snapshot(
                    config,
                    snapshot.as_ref(),
                    fresh_snapshot,
                    now_epoch_s,
                ));
            }
            Err(refresh_error) => {
                error = Some(refresh_error);
            }
        }
    }

    WeatherResolution {
        modifier: snapshot
            .as_ref()
            .and_then(|snapshot| snapshot_modifier(config, snapshot, now_epoch_s)),
        snapshot,
        next_refresh_at_epoch_s: if refresh_due {
            let failure_count = if error.is_some() {
                consecutive_refresh_failures.saturating_add(1)
            } else {
                0
            };
            Some(now_epoch_s.saturating_add(
                next_refresh_delay(config, error.as_ref(), failure_count).as_secs(),
            ))
        } else {
            next_refresh_at_epoch_s
        },
        error,
        refresh_attempted: refresh_due,
    }
}


#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::BTreeMap;

    use crate::config::{WeatherConfig, WeatherProvider as ConfigWeatherProvider};
    use crate::solar::Location;
    use crate::state::WeatherSnapshotMetadata;

    use super::{
        cloud_cover_to_multiplier, refresh_interval, resolve_modifier_with_provider,
        snapshot_modifier, snapshot_state, EnvironmentReader, WeatherError, WeatherProvider,
        WeatherRequest, WeatherSnapshot, WeatherSnapshotState,
    };

    #[test]
    fn feature_disabled_skips_fetch_and_returns_no_modifier() {
        let provider = FakeProvider::success(WeatherSnapshot {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: 65,
            temperature: 0.0,
            forecast: vec![],
        });

        let resolution = resolve_modifier_with_provider(
            &weather_config(false),
            &location(),
            None,
            1_800_000_000,
            None,
            false,
            0,
            &provider,
            &FakeEnvironment::default(),
        );

        assert_eq!(provider.calls(), 0);
        assert!(resolution.modifier.is_none());
        assert!(resolution.error.is_none());
        assert_eq!(resolution.next_refresh_at_epoch_s, None);
        assert!(!resolution.refresh_attempted);
    }

    #[test]
    fn missing_api_key_uses_cache_without_fetch() {
        let provider = FakeProvider::success(WeatherSnapshot {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: 100,
            temperature: 0.0,
            forecast: vec![],
        });
        let cached = WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: Some(80),
            smoothed_cloud_cover_percent: Some(70),
            temperature: Some(0.0),
            forecast: vec![],
        };

        let resolution = resolve_modifier_with_provider(
            &weather_config(true),
            &location(),
            Some(&cached),
            1_800_000_100,
            None,
            false,
            0,
            &provider,
            &FakeEnvironment::default(),
        );

        assert_eq!(provider.calls(), 0);
        assert_eq!(
            resolution
                .modifier
                .expect("cache should produce a modifier"),
            cloud_cover_to_multiplier(70, 0.75)
        );
        assert!(matches!(
            resolution.error,
            Some(WeatherError::MissingApiKey { .. })
        ));
    }

    #[test]
    fn environment_api_key_takes_precedence_over_config_key() {
        let provider = FakeProvider::assert_request_api_key(
            String::from("env-key"),
            WeatherSnapshot {
                provider: String::from("openweather"),
                observed_at_epoch_s: 1_800_000_000,
                cloud_cover_percent: 40,
                temperature: 0.0,
                forecast: vec![],
            },
        );
        let mut config = weather_config(true);
        config.api_key = Some(String::from("config-key"));

        let resolution = resolve_modifier_with_provider(
            &config,
            &location(),
            None,
            1_800_000_000,
            None,
            false,
            0,
            &provider,
            &FakeEnvironment::with([(
                String::from("OPENWEATHER_API_KEY"),
                String::from("env-key"),
            )]),
        );

        assert_eq!(provider.calls(), 1);
        assert!(resolution.error.is_none());
        assert!(resolution.modifier.is_some());
    }

    #[test]
    fn cache_usage_skips_fetch_until_refresh_is_due() {
        let provider = FakeProvider::success(WeatherSnapshot {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: 0,
            temperature: 0.0,
            forecast: vec![],
        });
        let cached = WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: Some(60),
            smoothed_cloud_cover_percent: Some(60),
            temperature: Some(0.0),
            forecast: vec![],
        };

        let resolution = resolve_modifier_with_provider(
            &weather_config(true),
            &location(),
            Some(&cached),
            1_800_000_100,
            Some(1_800_001_000),
            false,
            0,
            &provider,
            &FakeEnvironment::with([(
                String::from("OPENWEATHER_API_KEY"),
                String::from("env-key"),
            )]),
        );

        assert_eq!(provider.calls(), 0);
        assert!(resolution.modifier.is_some());
        assert_eq!(resolution.next_refresh_at_epoch_s, Some(1_800_001_000));
    }

    #[test]
    fn refresh_failure_uses_fresh_cache_then_falls_back_to_pure_solar_after_expiry() {
        let provider = FakeProvider::failure(WeatherError::Transport {
            provider: "openweather",
            message: String::from("HTTPS request failed"),
        });
        let cached = WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: Some(90),
            smoothed_cloud_cover_percent: Some(90),
            temperature: Some(0.0),
            forecast: vec![],
        };
        let environment =
            FakeEnvironment::with([(String::from("OPENWEATHER_API_KEY"), String::from("env-key"))]);

        let fresh = resolve_modifier_with_provider(
            &weather_config(true),
            &location(),
            Some(&cached),
            1_800_000_100,
            None,
            false,
            0,
            &provider,
            &environment,
        );
        assert_eq!(provider.calls(), 1);
        assert!(fresh.modifier.is_some());

        let expired = resolve_modifier_with_provider(
            &weather_config(true),
            &location(),
            Some(&cached),
            1_800_004_000,
            None,
            false,
            1,
            &provider,
            &environment,
        );
        assert_eq!(provider.calls(), 2);
        assert!(expired.modifier.is_none());
    }

    #[test]
    fn bounded_multiplier_behavior_smooths_large_changes_and_stays_within_range() {
        let provider = FakeProvider::success(WeatherSnapshot {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: 0,
            temperature: 0.0,
            forecast: vec![],
        });
        let cached = WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_799_999_900,
            cloud_cover_percent: Some(100),
            smoothed_cloud_cover_percent: Some(100),
            temperature: Some(0.0),
            forecast: vec![],
        };
        let resolution = resolve_modifier_with_provider(
            &weather_config(true),
            &location(),
            Some(&cached),
            1_800_000_000,
            None,
            false,
            0,
            &provider,
            &FakeEnvironment::with([(
                String::from("OPENWEATHER_API_KEY"),
                String::from("env-key"),
            )]),
        );

        let updated_snapshot = resolution
            .snapshot
            .expect("successful refresh should update cached weather");
        let smoothed_cloud_cover_percent = updated_snapshot
            .smoothed_cloud_cover_percent
            .expect("smoothed cloud cover should be stored");
        let modifier = resolution
            .modifier
            .expect("updated weather should produce a modifier");

        assert_eq!(smoothed_cloud_cover_percent, 50);
        assert!((0.75..=1.0).contains(&modifier));
        assert_eq!(modifier, cloud_cover_to_multiplier(50, 0.75));
    }

    #[test]
    fn snapshot_modifier_respects_cache_ttl() {
        let config = weather_config(true);
        let snapshot = WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: Some(60),
            smoothed_cloud_cover_percent: Some(55),
            temperature: Some(0.0),
            forecast: vec![],
        };

        assert!(snapshot_modifier(&config, &snapshot, 1_800_000_100).is_some());
        assert!(snapshot_modifier(
            &config,
            &snapshot,
            1_800_000_000 + refresh_interval(&config).as_secs() * 3
        )
        .is_none());
    }

    #[test]
    fn transport_failures_retry_before_full_refresh_interval() {
        let provider = FakeProvider::failure(WeatherError::Transport {
            provider: "openweather",
            message: String::from("temporary network failure"),
        });

        let resolution = resolve_modifier_with_provider(
            &weather_config(true),
            &location(),
            None,
            1_800_000_000,
            None,
            false,
            0,
            &provider,
            &FakeEnvironment::with([(
                String::from("OPENWEATHER_API_KEY"),
                String::from("env-key"),
            )]),
        );

        assert_eq!(provider.calls(), 1);
        assert_eq!(resolution.next_refresh_at_epoch_s, Some(1_800_000_060));
        assert!(resolution.refresh_attempted);
    }

    #[test]
    fn auth_failures_use_normal_refresh_interval() {
        let provider = FakeProvider::failure(WeatherError::HttpStatus {
            provider: "openweather",
            status: 401,
        });

        let resolution = resolve_modifier_with_provider(
            &weather_config(true),
            &location(),
            None,
            1_800_000_000,
            None,
            false,
            0,
            &provider,
            &FakeEnvironment::with([(
                String::from("OPENWEATHER_API_KEY"),
                String::from("env-key"),
            )]),
        );

        assert_eq!(
            resolution.next_refresh_at_epoch_s,
            Some(1_800_000_000 + refresh_interval(&weather_config(true)).as_secs())
        );
    }

    #[test]
    fn forced_refresh_ignores_existing_deadline() {
        let provider = FakeProvider::success(WeatherSnapshot {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: 35,
            temperature: 0.0,
            forecast: vec![],
        });

        let resolution = resolve_modifier_with_provider(
            &weather_config(true),
            &location(),
            None,
            1_800_000_000,
            Some(1_800_000_900),
            true,
            0,
            &provider,
            &FakeEnvironment::with([(
                String::from("OPENWEATHER_API_KEY"),
                String::from("env-key"),
            )]),
        );

        assert_eq!(provider.calls(), 1);
        assert!(resolution.refresh_attempted);
        assert_eq!(
            resolution.next_refresh_at_epoch_s,
            Some(1_800_000_000 + refresh_interval(&weather_config(true)).as_secs())
        );
    }

    #[test]
    fn snapshot_state_distinguishes_missing_stale_and_ready_data() {
        let config = weather_config(true);
        let ready_snapshot = WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: Some(60),
            smoothed_cloud_cover_percent: Some(55),
            temperature: Some(0.0),
            forecast: vec![],
        };
        let incomplete_snapshot = WeatherSnapshotMetadata {
            provider: String::from("openweather"),
            observed_at_epoch_s: 1_800_000_000,
            cloud_cover_percent: None,
            smoothed_cloud_cover_percent: None,
            temperature: Some(0.0),
            forecast: vec![],
        };

        assert_eq!(
            snapshot_state(&config, None, 1_800_000_000),
            WeatherSnapshotState::Missing
        );
        assert_eq!(
            snapshot_state(&config, Some(&ready_snapshot), 1_800_000_100),
            WeatherSnapshotState::Ready
        );
        assert_eq!(
            snapshot_state(
                &config,
                Some(&ready_snapshot),
                1_800_000_000 + refresh_interval(&config).as_secs() * 3
            ),
            WeatherSnapshotState::Stale
        );
        assert_eq!(
            snapshot_state(&config, Some(&incomplete_snapshot), 1_800_000_100),
            WeatherSnapshotState::Incomplete
        );
    }

    fn weather_config(enabled: bool) -> WeatherConfig {
        WeatherConfig {
            enabled,
            provider: Some(ConfigWeatherProvider::OpenWeather),
            api_key_env: Some(String::from("OPENWEATHER_API_KEY")),
            api_key: None,
            refresh_minutes: 30,
            min_multiplier: 0.75,
        }
    }

    fn location() -> Location {
        Location::from_timezone_name(41.0082, 28.9784, "Europe/Istanbul")
            .expect("timezone should parse")
    }

    #[derive(Default)]
    struct FakeEnvironment {
        values: BTreeMap<String, String>,
    }

    impl FakeEnvironment {
        fn with(values: [(String, String); 1]) -> Self {
            Self {
                values: values.into_iter().collect(),
            }
        }
    }

    impl EnvironmentReader for FakeEnvironment {
        fn get(&self, key: &str) -> Option<String> {
            self.values.get(key).cloned()
        }
    }

    struct FakeProvider {
        calls: Cell<usize>,
        result: Result<WeatherSnapshot, WeatherError>,
        expected_api_key: Option<String>,
    }

    impl FakeProvider {
        fn success(snapshot: WeatherSnapshot) -> Self {
            Self {
                calls: Cell::new(0),
                result: Ok(snapshot),
                expected_api_key: None,
            }
        }

        fn failure(error: WeatherError) -> Self {
            Self {
                calls: Cell::new(0),
                result: Err(error),
                expected_api_key: None,
            }
        }

        fn assert_request_api_key(expected_api_key: String, snapshot: WeatherSnapshot) -> Self {
            Self {
                calls: Cell::new(0),
                result: Ok(snapshot),
                expected_api_key: Some(expected_api_key),
            }
        }

        fn calls(&self) -> usize {
            self.calls.get()
        }
    }

    impl WeatherProvider for FakeProvider {
        fn name(&self) -> &'static str {
            "openweather"
        }

        fn fetch_snapshot(
            &self,
            _request: &WeatherRequest,
        ) -> Result<WeatherSnapshot, WeatherError> {
            self.calls.set(self.calls.get() + 1);
            if let Some(expected_api_key) = self.expected_api_key.as_ref() {
                assert_eq!(_request.api_key, *expected_api_key);
            }
            self.result.clone()
        }
    }
}
