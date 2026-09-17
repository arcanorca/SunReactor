use crate::ipc;
use crate::runtime::orchestrator::DaemonRuntime;
use crate::runtime::topology;
use crate::weather;

impl DaemonRuntime {
    #[allow(clippy::too_many_lines)]
    pub(super) fn status_response(&self, now_epoch_s: u64) -> ipc::StatusResponse {
        let control = self.state.effective_control(now_epoch_s);
        let weather = self.weather_status(now_epoch_s);
        let topology = self
            .last_capabilities
            .as_ref()
            .map(|capabilities| topology::reconcile(&self.config.monitors, capabilities));
        let monitors = self
            .config
            .monitors
            .iter()
            .enumerate()
            .map(|(index, monitor)| {
                let monitor_state = self.state.monitor(&monitor.logical_id);
                ipc::MonitorStatus {
                    logical_id: monitor.logical_id.clone(),
                    backend: monitor.backend,
                    enabled: monitor.enabled,
                    override_percent: control.monitor_override_percent(&monitor.logical_id),
                    last_applied_percent: monitor_state
                        .and_then(|state| state.last_applied_percent),
                    last_applied_at_epoch_s: monitor_state
                        .and_then(|state| state.last_applied_at_epoch_s),
                    backoff_until_epoch_s: monitor_state
                        .and_then(|state| state.backoff.as_ref())
                        .and_then(|backoff| {
                            if backoff.backend == monitor.backend {
                                backoff.suppress_until_epoch_s
                            } else {
                                None
                            }
                        }),
                    topology: topology
                        .as_ref()
                        .and_then(|actions| actions.get(index))
                        .map(|action| action.name().to_owned()),
                }
            })
            .collect();

        let now_utc = chrono::DateTime::from_timestamp(now_epoch_s as i64, 0).unwrap_or_default();
        let events = crate::solar::local_datetime_at_utc(now_utc, &self.config.location)
            .ok()
            .and_then(|now_local| {
                crate::solar::get_sun_events(now_local.date_naive(), &self.config.location).ok()
            });

        ipc::StatusResponse {
            daemon_alive: true,
            config_path: self.config.path.display().to_string(),
            tick_seconds: self.config.daemon.tick_seconds,
            dry_run: self.config.daemon.dry_run,
            suspended: control.suspended,
            desktop_idle_dimmed: control.desktop_idle_dimmed,
            suspend_until_epoch_s: control.suspend_until_epoch_s,
            manual_override_active: control.manual_override_active,
            per_monitor_override_until_epoch_s: control.per_monitor_override_until_epoch_s,
            global_override_percent: control.global_override_percent,
            global_override_until_epoch_s: control.global_override_until_epoch_s,
            configured_monitors: self.config.monitors.len() as u32,
            stateful_monitors: self.state.monitors.len() as u32,
            weather,
            monitors,
            solar_elevation: self.last_solar_elevation,
            now_epoch_s,
            sunrise_epoch_s: events
                .as_ref()
                .map(|event| event.sunrise.timestamp() as u64),
            sunset_epoch_s: events.as_ref().map(|event| event.sunset.timestamp() as u64),
            lunar_phase: Some(crate::solar::calculate_lunar_phase(now_utc)),
        }
    }

    fn weather_status(&self, now_epoch_s: u64) -> Option<ipc::WeatherStatus> {
        if !self.config.weather.enabled {
            return None;
        }

        let service = self.weather_engine.latest_status().ok();
        let service_has_snapshot = service
            .as_ref()
            .is_some_and(|service| service.snapshot.is_some());
        let snapshot_owned = service
            .as_ref()
            .and_then(|service| service.snapshot.clone())
            .or_else(|| self.state.weather.clone());
        let snapshot = snapshot_owned.as_ref();
        let snapshot_state = weather::snapshot_state(&self.config.weather, snapshot, now_epoch_s);
        let state = service.as_ref().map_or_else(
            || match snapshot_state {
                weather::WeatherSnapshotState::Ready => weather::WeatherState::Ready,
                weather::WeatherSnapshotState::Stale => weather::WeatherState::Stale,
                weather::WeatherSnapshotState::Missing
                | weather::WeatherSnapshotState::Incomplete => weather::WeatherState::Loading,
            },
            |service| service.state,
        );
        let multiplier = snapshot.and_then(|weather| {
            weather::snapshot_modifier(&self.config.weather, weather, now_epoch_s)
        });

        Some(ipc::WeatherStatus {
            enabled: true,
            state,
            // Loading is a transport state, not a statement that a
            // last-known-good snapshot is unusable.
            active: multiplier.is_some(),
            stale: state == weather::WeatherState::Stale
                || snapshot_state == weather::WeatherSnapshotState::Stale,
            provider: snapshot
                .map(|weather| weather.provider.trim().to_owned())
                .filter(|provider| !provider.is_empty()),
            fetched_at_epoch_s: snapshot.map(|weather| weather.fetched_at_epoch_s),
            valid_at_epoch_s: snapshot.and_then(|weather| {
                (weather.valid_at_epoch_s != 0).then_some(weather.valid_at_epoch_s)
            }),
            source_kind: snapshot.map(|weather| weather.source_kind),
            last_refresh_attempt_epoch_s: service
                .as_ref()
                .filter(|_| service_has_snapshot)
                .and_then(|service| service.last_attempted_at_epoch_s)
                .or(self.weather_refresh.last_attempted_at_epoch_s),
            next_refresh_at_epoch_s: service
                .as_ref()
                .filter(|_| service_has_snapshot)
                .and_then(|service| service.next_refresh_at_epoch_s)
                .or(self.weather_refresh.next_refresh_at_epoch_s),
            consecutive_failures: service
                .as_ref()
                .filter(|_| service_has_snapshot)
                .map_or(self.weather_refresh.consecutive_failures, |service| {
                    service.consecutive_failures
                }),
            last_error: service
                .as_ref()
                .filter(|_| service_has_snapshot)
                .and_then(|service| service.last_error.clone())
                .or_else(|| self.weather_refresh.last_error.clone()),
            cloud_cover_percent: snapshot.and_then(|weather| weather.cloud_cover_percent),
            temperature: snapshot.and_then(|weather| weather.temperature),
            condition: snapshot
                .map(|weather| weather.condition)
                .unwrap_or_default(),
            condition_description: snapshot
                .and_then(|weather| weather.condition_description.clone()),
            day_phase: snapshot.and_then(|weather| weather.day_phase),
            forecast: snapshot
                .map(|weather| weather.forecast.clone())
                .unwrap_or_default(),
            multiplier,
            details: snapshot.map(|weather| weather.details).unwrap_or_default(),
        })
    }
}
