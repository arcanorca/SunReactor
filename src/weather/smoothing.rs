use super::types::{WeatherSnapshot, WeatherSnapshotState};
use crate::config::WeatherConfig;
use crate::state::WeatherSnapshotMetadata;
use std::time::Duration;

const WEATHER_CACHE_TTL_MULTIPLIER: u64 = 2;
/// After the cache TTL the cloud dimming does not snap to 1.0 (a visible
/// brightness step): it fades linearly back to neutral over this long.
pub const WEATHER_EXPIRY_DECAY: Duration = Duration::from_mins(30);
#[must_use]
pub fn snapshot_state(
    config: &WeatherConfig,
    snapshot: Option<&WeatherSnapshotMetadata>,
    now_epoch_s: u64,
) -> WeatherSnapshotState {
    let Some(snapshot) = snapshot else {
        return WeatherSnapshotState::Missing;
    };

    if !cache_is_fresh(config, snapshot, now_epoch_s) {
        return WeatherSnapshotState::Stale;
    }

    if snapshot
        .smoothed_cloud_cover_percent
        .or(snapshot.cloud_cover_percent)
        .is_none()
    {
        return WeatherSnapshotState::Incomplete;
    }

    WeatherSnapshotState::Ready
}
#[must_use]
pub fn snapshot_modifier(
    config: &WeatherConfig,
    snapshot: &WeatherSnapshotMetadata,
    now_epoch_s: u64,
) -> Option<f64> {
    if !config.enabled {
        return None;
    }
    let state = snapshot_state(config, Some(snapshot), now_epoch_s);
    if !matches!(
        state,
        WeatherSnapshotState::Ready | WeatherSnapshotState::Stale
    ) {
        return None;
    }

    let cloud_cover_percent = snapshot
        .smoothed_cloud_cover_percent
        .or(snapshot.cloud_cover_percent)?;
    let multiplier = cloud_cover_to_multiplier(cloud_cover_percent, config.min_multiplier);
    if state == WeatherSnapshotState::Ready {
        return Some(multiplier);
    }
    expired_multiplier(config, snapshot, multiplier, now_epoch_s)
}

/// Linear fade from `multiplier` back to 1.0 across [`WEATHER_EXPIRY_DECAY`]
/// after the TTL; `None` once neutral. A future-dated snapshot (the clock
/// stepped back) is not trusted here; the refresh worker clamps and persists
/// its timestamp so it then decays from that moment.
fn expired_multiplier(
    config: &WeatherConfig,
    snapshot: &WeatherSnapshotMetadata,
    multiplier: f64,
    now_epoch_s: u64,
) -> Option<f64> {
    if snapshot.provider.trim().is_empty() {
        return None;
    }
    let age = now_epoch_s.checked_sub(snapshot.fetched_at_epoch_s)?;
    let past_ttl = age.checked_sub(cache_ttl(config).as_secs())?;
    let decay = WEATHER_EXPIRY_DECAY.as_secs();
    if past_ttl >= decay {
        return None;
    }
    let progress = past_ttl as f64 / decay as f64;
    Some(multiplier + (1.0 - multiplier) * progress)
}
#[must_use]
pub fn refresh_interval(config: &WeatherConfig) -> Duration {
    Duration::from_secs(u64::from(config.refresh_minutes).saturating_mul(60))
}
#[must_use]
pub fn cache_ttl(config: &WeatherConfig) -> Duration {
    Duration::from_secs(
        refresh_interval(config)
            .as_secs()
            .saturating_mul(WEATHER_CACHE_TTL_MULTIPLIER),
    )
}
#[must_use]
pub fn cloud_cover_to_multiplier(cloud_cover_percent: u8, min_multiplier: f64) -> f64 {
    if cloud_cover_percent == 0 {
        return 1.0;
    }
    let minimum = min_multiplier.clamp(0.0, 1.0);
    let cloud_factor = f64::from(cloud_cover_percent.min(100)) / 100.0;
    (1.0 - cloud_factor * (1.0 - minimum)).clamp(minimum, 1.0)
}
pub(crate) fn merge_snapshot(
    config: &WeatherConfig,
    previous: Option<&WeatherSnapshotMetadata>,
    fresh_snapshot: WeatherSnapshot,
    now_epoch_s: u64,
) -> WeatherSnapshotMetadata {
    let previous_smoothed = previous
        .filter(|snapshot| cache_is_fresh(config, snapshot, now_epoch_s))
        .filter(|snapshot| snapshot.provider.trim() == fresh_snapshot.provider)
        .and_then(|snapshot| {
            snapshot
                .smoothed_cloud_cover_percent
                .or(snapshot.cloud_cover_percent)
        });

    let smoothed_cloud_cover_percent =
        smooth_cloud_cover(previous_smoothed, fresh_snapshot.cloud_cover_percent);

    WeatherSnapshotMetadata {
        provider: fresh_snapshot.provider,
        fetched_at_epoch_s: fresh_snapshot.fetched_at_epoch_s.min(now_epoch_s),
        valid_at_epoch_s: fresh_snapshot.valid_at_epoch_s,
        source_kind: fresh_snapshot.source_kind,
        cloud_cover_percent: Some(fresh_snapshot.cloud_cover_percent),
        smoothed_cloud_cover_percent: Some(smoothed_cloud_cover_percent),
        temperature: Some(fresh_snapshot.temperature),
        condition: fresh_snapshot.condition,
        condition_description: fresh_snapshot.condition_description,
        day_phase: fresh_snapshot.day_phase,
        forecast: fresh_snapshot.forecast,
        details: fresh_snapshot.details,
    }
}
pub(crate) fn cache_is_fresh(
    config: &WeatherConfig,
    snapshot: &WeatherSnapshotMetadata,
    now_epoch_s: u64,
) -> bool {
    let Some(age_seconds) = now_epoch_s.checked_sub(snapshot.fetched_at_epoch_s) else {
        return false;
    };
    !snapshot.provider.trim().is_empty() && age_seconds <= cache_ttl(config).as_secs()
}
pub(crate) fn smooth_cloud_cover(
    previous_cloud_cover_percent: Option<u8>,
    fresh_cloud_cover_percent: u8,
) -> u8 {
    match previous_cloud_cover_percent {
        Some(previous_cloud_cover_percent) => (u16::from(previous_cloud_cover_percent)
            + u16::from(fresh_cloud_cover_percent))
        .div_ceil(2) as u8,
        None => fresh_cloud_cover_percent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_dated_snapshot_is_stale() {
        let config = WeatherConfig::default();
        let snapshot = WeatherSnapshotMetadata {
            provider: String::from("test"),
            fetched_at_epoch_s: 1_800_000_300,
            cloud_cover_percent: Some(50),
            ..WeatherSnapshotMetadata::default()
        };

        assert!(!cache_is_fresh(&config, &snapshot, 1_800_000_000));
        assert_eq!(snapshot_modifier(&config, &snapshot, 1_800_000_000), None);
    }

    fn cloudy(fetched_at_epoch_s: u64) -> WeatherSnapshotMetadata {
        WeatherSnapshotMetadata {
            provider: String::from("test"),
            fetched_at_epoch_s,
            cloud_cover_percent: Some(100),
            smoothed_cloud_cover_percent: Some(100),
            ..WeatherSnapshotMetadata::default()
        }
    }

    #[test]
    fn cloud_dimming_fades_out_after_the_ttl_instead_of_stepping() {
        let config = WeatherConfig {
            enabled: true,
            min_multiplier: 0.6,
            ..WeatherConfig::default()
        };
        let snapshot = cloudy(1_800_000_000);
        let ttl = cache_ttl(&config).as_secs();
        let at = |past: u64| snapshot_modifier(&config, &snapshot, 1_800_000_000 + ttl + past);

        let fresh = at(0).expect("still fresh at the TTL");
        assert!((fresh - 0.6).abs() < 1e-9);
        let mut previous = fresh;
        for minute in 1..30 {
            let value = at(minute * 60).expect("decaying");
            assert!(value > previous && value < 1.0, "monotonic, never a jump");
            assert!(value - previous < 0.02, "no step larger than 2 points");
            previous = value;
        }
        assert!(
            (at(15 * 60).unwrap() - 0.8).abs() < 1e-9,
            "halfway at 15 min"
        );
        assert_eq!(at(WEATHER_EXPIRY_DECAY.as_secs()), None);
        assert_eq!(at(WEATHER_EXPIRY_DECAY.as_secs() + 3_600), None);
    }
}
