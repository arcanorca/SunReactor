use crate::config::WeatherConfig;
use crate::solar::Location;
use crate::state::WeatherSnapshotMetadata;
use crate::weather::{resolve_modifier_with_provider, OpenWeatherProvider, ProcessEnvironment};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub struct WeatherEngine {
    config: WeatherConfig,
    location: Location,
    cache: Arc<RwLock<Option<WeatherSnapshotMetadata>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
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
        let mut engine = Self {
            config,
            location,
            cache: Arc::new(RwLock::new(initial_snapshot)),
            shutdown_tx: None,
            thread_handle: None,
        };
        engine.start_thread();
        engine
    }

    /// Attempts to read the latest snapshot from the cache.
    /// Returns `Ok(Option<WeatherSnapshotMetadata>)` if the lock was acquired,
    /// or `Err(())` if the lock is currently held by the background thread.
    pub fn latest_snapshot(&self) -> Result<Option<WeatherSnapshotMetadata>, ()> {
        self.cache.try_read().map(|g| g.clone()).map_err(|_| ())
    }

    pub fn sync_config(&mut self, new_config: &WeatherConfig, new_location: &Location) {
        if &self.config != new_config || &self.location != new_location {
            self.config = new_config.clone();
            self.location = new_location.clone();
            self.stop_thread();
            self.start_thread();
        }
    }

    fn stop_thread(&mut self) {
        self.shutdown_tx.take();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    fn start_thread(&mut self) {
        if !self.config.enabled {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.shutdown_tx = Some(tx);

        let config = self.config.clone();
        let location = self.location.clone();
        let cache = Arc::clone(&self.cache);

        self.thread_handle = Some(thread::spawn(move || {
            let provider = OpenWeatherProvider;
            let mut consecutive_failures = 0;
            let mut next_refresh_at_epoch_s = None;

            loop {
                let now_epoch_s = chrono::Utc::now().timestamp().max(0) as u64;
                let current_cache = cache.read().unwrap().clone();

                let resolution = resolve_modifier_with_provider(
                    &config,
                    &location,
                    current_cache.as_ref(),
                    now_epoch_s,
                    next_refresh_at_epoch_s,
                    false,
                    consecutive_failures,
                    &provider,
                    &ProcessEnvironment,
                );

                if let Some(error) = &resolution.error {
                    tracing::error!(error=%error, "weather_fetch_failed");
                    consecutive_failures += 1;
                } else if resolution.refresh_attempted {
                    consecutive_failures = 0;
                }

                if resolution.snapshot != current_cache {
                    *cache.write().unwrap() = resolution.snapshot;
                }

                next_refresh_at_epoch_s = resolution.next_refresh_at_epoch_s;

                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Timeout is expected, continue to next iteration
                    }
                }
            }
        }));
    }
}
