use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::config::WeatherConfig;
use crate::solar::Location;
use crate::state::WeatherSnapshotMetadata;
use crate::weather::{
    resolve_modifier_with_provider, snapshot_state, OpenWeatherProvider, ProcessEnvironment,
    WeatherResolution, WeatherServiceStatus, WeatherSnapshotState, WeatherState,
};

const IDLE_RECHECK: Duration = Duration::from_mins(1);

enum WeatherCommand {
    Refresh,
    Shutdown,
}

/// Owns weather I/O and its observable state. The TUI and daemon only read a
/// snapshot from this service; no network request occurs on their hot paths.
#[derive(Debug)]
pub struct WeatherEngine {
    config: WeatherConfig,
    location: Location,
    status: Arc<RwLock<WeatherServiceStatus>>,
    command_tx: Option<mpsc::SyncSender<WeatherCommand>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl Drop for WeatherEngine {
    fn drop(&mut self) {
        self.stop_thread();
    }
}

impl WeatherEngine {
    pub fn new(
        config: WeatherConfig,
        location: Location,
        initial_snapshot: Option<WeatherSnapshotMetadata>,
    ) -> Self {
        let status = if config.enabled {
            WeatherServiceStatus::loading(initial_snapshot)
        } else {
            WeatherServiceStatus::disabled(initial_snapshot)
        };
        let mut engine = Self {
            config,
            location,
            status: Arc::new(RwLock::new(status)),
            command_tx: None,
            thread_handle: None,
        };
        engine.start_thread();
        engine
    }

    /// Reads service state without blocking rendering or the daemon loop.
    #[allow(clippy::result_unit_err)]
    pub fn latest_status(&self) -> Result<WeatherServiceStatus, ()> {
        self.status
            .try_read()
            .map(|state| state.clone())
            .map_err(|_| ())
    }

    /// Schedules an immediate provider request. The caller receives an
    /// acknowledgement immediately; completion appears through `latest_status`.
    pub fn request_refresh(&mut self) -> bool {
        if !self.config.enabled {
            return false;
        }
        let Some(tx) = &self.command_tx else {
            return false;
        };
        if tx.try_send(WeatherCommand::Refresh).is_err() {
            return false;
        }
        set_loading(&self.status);
        true
    }

    /// Restarts the worker when a key, provider, or location changes. A new
    /// worker always performs its first request immediately.
    pub fn sync_config(&mut self, new_config: &WeatherConfig, new_location: &Location) {
        if !needs_worker_restart(&self.config, &self.location, new_config, new_location) {
            return;
        }

        let old_snapshot = self.latest_status().ok().and_then(|status| status.snapshot);
        let location_changed = self.location != *new_location;
        self.config = new_config.clone();
        self.location = new_location.clone();
        self.stop_thread();
        let initial_snapshot = if location_changed { None } else { old_snapshot };
        self.status = Arc::new(RwLock::new(if self.config.enabled {
            WeatherServiceStatus::loading(initial_snapshot)
        } else {
            WeatherServiceStatus::disabled(None)
        }));
        self.start_thread();
    }

    fn stop_thread(&mut self) {
        if let Some(tx) = self.command_tx.take() {
            let _ = tx.try_send(WeatherCommand::Shutdown);
        }
        // A provider request has its own bounded timeout. Do not block config
        // reload while an old worker is finishing that request; it owns the old
        // Arc and cannot publish into the replacement service state.
        self.thread_handle.take();
    }

    fn start_thread(&mut self) {
        if !self.config.enabled {
            return;
        }

        let (tx, rx) = mpsc::sync_channel(1);
        self.command_tx = Some(tx);
        let config = self.config.clone();
        let location = self.location.clone();
        let status = Arc::clone(&self.status);

        self.thread_handle = Some(thread::spawn(move || {
            run_weather_worker(config, location, status, rx);
        }));
    }
}

fn needs_worker_restart(
    current_config: &WeatherConfig,
    current_location: &Location,
    next_config: &WeatherConfig,
    next_location: &Location,
) -> bool {
    current_config != next_config || current_location != next_location
}

#[allow(clippy::needless_pass_by_value)]
fn run_weather_worker(
    config: WeatherConfig,
    location: Location,
    status: Arc<RwLock<WeatherServiceStatus>>,
    rx: mpsc::Receiver<WeatherCommand>,
) {
    let provider = OpenWeatherProvider;
    let mut consecutive_failures = 0;
    let mut next_refresh_at_epoch_s = None;
    let mut force_refresh = true;

    loop {
        let now_epoch_s = chrono::Utc::now().timestamp().max(0) as u64;
        let cached = status
            .read()
            .ok()
            .and_then(|current| current.snapshot.clone());

        if force_refresh {
            set_loading(&status);
        }

        let resolution = resolve_modifier_with_provider(
            &config,
            &location,
            cached.as_ref(),
            now_epoch_s,
            next_refresh_at_epoch_s,
            force_refresh,
            consecutive_failures,
            &provider,
            &ProcessEnvironment,
        );
        let next_state = status_after_resolution(&config, &resolution, now_epoch_s);
        let did_fail = resolution.error.is_some();
        consecutive_failures = if did_fail {
            consecutive_failures.saturating_add(1)
        } else if resolution.refresh_attempted {
            0
        } else {
            consecutive_failures
        };
        next_refresh_at_epoch_s = resolution.next_refresh_at_epoch_s;

        if let Some(error) = &resolution.error {
            tracing::warn!(state = ?next_state, error = %error, "weather_fetch_failed");
        }

        if let Ok(mut current) = status.write() {
            current.state = next_state;
            current.snapshot = resolution.snapshot;
            current.last_attempted_at_epoch_s = resolution.refresh_attempted.then_some(now_epoch_s);
            current.next_refresh_at_epoch_s = next_refresh_at_epoch_s;
            current.consecutive_failures = consecutive_failures;
            current.last_error = resolution.error.map(|error| error.to_string());
        }

        force_refresh = false;
        let wait = next_refresh_at_epoch_s
            .and_then(|at| at.checked_sub(now_epoch_s))
            .map_or(IDLE_RECHECK, Duration::from_secs);
        match rx.recv_timeout(wait) {
            Ok(WeatherCommand::Refresh) => force_refresh = true,
            Ok(WeatherCommand::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

fn set_loading(status: &RwLock<WeatherServiceStatus>) {
    if let Ok(mut current) = status.write() {
        current.state = WeatherState::Loading;
        current.last_error = None;
    }
}

fn status_after_resolution(
    config: &WeatherConfig,
    resolution: &WeatherResolution,
    now_epoch_s: u64,
) -> WeatherState {
    if let Some(error) = &resolution.error {
        return WeatherState::from_error(error);
    }

    match snapshot_state(config, resolution.snapshot.as_ref(), now_epoch_s) {
        WeatherSnapshotState::Ready => WeatherState::Ready,
        WeatherSnapshotState::Missing | WeatherSnapshotState::Incomplete => WeatherState::Loading,
        WeatherSnapshotState::Stale => WeatherState::Stale,
    }
}

#[cfg(test)]
mod tests {
    use crate::config::WeatherConfig;
    use crate::solar::Location;

    use super::needs_worker_restart;

    #[test]
    fn changing_an_api_key_restarts_the_worker_for_an_immediate_fetch() {
        let current = WeatherConfig::default();
        let mut changed = current.clone();
        changed.api_key = Some(String::from("new-key"));
        changed.api_key_env = None;
        let location = Location::from_timezone_name(41.0, 29.0, "Europe/Istanbul")
            .expect("test location should be valid");

        assert!(needs_worker_restart(
            &current, &location, &changed, &location
        ));
    }
}
