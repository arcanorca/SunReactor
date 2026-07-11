use super::types::{WeatherSnapshot, WeatherSnapshotState};
use crate::config::WeatherConfig;
use crate::state::WeatherSnapshotMetadata;
use std::time::Duration;

const WEATHER_CACHE_TTL_MULTIPLIER: u64 = 2;
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
    if !config.enabled
        || snapshot_state(config, Some(snapshot), now_epoch_s) != WeatherSnapshotState::Ready
    {
        return None;
    }

    let cloud_cover_percent = snapshot
        .smoothed_cloud_cover_percent
        .or(snapshot.cloud_cover_percent)?;

    Some(cloud_cover_to_multiplier(
        cloud_cover_percent,
        config.min_multiplier,
    ))
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
        observed_at_epoch_s: fresh_snapshot.observed_at_epoch_s,
        cloud_cover_percent: Some(fresh_snapshot.cloud_cover_percent),
        smoothed_cloud_cover_percent: Some(smoothed_cloud_cover_percent),
        temperature: Some(fresh_snapshot.temperature),
        forecast: fresh_snapshot.forecast,
    }
}
pub(crate) fn cache_is_fresh(
    config: &WeatherConfig,
    snapshot: &WeatherSnapshotMetadata,
    now_epoch_s: u64,
) -> bool {
    !snapshot.provider.trim().is_empty()
        && now_epoch_s.saturating_sub(snapshot.observed_at_epoch_s) <= cache_ttl(config).as_secs()
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
