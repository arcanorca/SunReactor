use crate::solar::Location;
use crate::weather;

#[derive(Debug, Clone, Default)]
pub struct WeatherRefreshState {
    pub next_refresh_at_epoch_s: Option<u64>,
    pub last_attempted_at_epoch_s: Option<u64>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

impl super::orchestrator::DaemonRuntime {
    pub(super) fn refresh_weather_modifier(
        &mut self,
        _location: &Location,
        now_epoch_s: u64,
        force_refresh: bool,
    ) -> Option<f64> {
        if !self.config.weather.enabled {
            return None;
        }

        if force_refresh {
            let _ = self.weather_engine.request_refresh();
        }

        if let Ok(service) = self.weather_engine.latest_status() {
            // A newly started worker is Loading before it has published its
            // first result. Keep a valid persisted snapshot during that gap so
            // policy does not abruptly fall back to pure solar input.
            let snapshot = service
                .snapshot
                .clone()
                .or_else(|| self.state.weather.clone());
            if let Some(snapshot) = snapshot.as_ref() {
                self.state.weather = Some(snapshot.clone());
            }

            // The service owns refresh metadata once it has published a
            // snapshot. Until then, retain persisted metadata alongside the
            // explicit service state (for example, NoApiKey) so a startup
            // attempt cannot erase the last-known-good context.
            if service.snapshot.is_some() {
                self.weather_refresh.next_refresh_at_epoch_s = service.next_refresh_at_epoch_s;
                self.weather_refresh.last_attempted_at_epoch_s = service.last_attempted_at_epoch_s;
                self.weather_refresh.consecutive_failures = service.consecutive_failures;
                self.weather_refresh.last_error = service.last_error;
            }

            return snapshot.as_ref().and_then(|snapshot| {
                weather::snapshot_modifier(&self.config.weather, snapshot, now_epoch_s)
            });
        }

        self.state.weather.as_ref().and_then(|snapshot| {
            weather::snapshot_modifier(&self.config.weather, snapshot, now_epoch_s)
        })
    }
}
