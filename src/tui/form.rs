use tui_input::Input;

use crate::config::Config;

use super::{ActiveInputKind, Tab};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WeatherApiKeySource {
    Missing,
    Config,
    Environment,
}

pub(crate) struct FormState {
    pub(crate) city_search_input: Input,
    pub(crate) city_search_results: Vec<usize>,
    pub(crate) city_search_selected_index: usize,
    pub(crate) lat_input: Input,
    pub(crate) lon_input: Input,
    pub(crate) timezone_input: Input,
    pub(crate) api_key_input: Input,
    pub(crate) desktop_idle_timeout_minutes_input: Input,
    pub(crate) suspend_minutes_input: Input,
    pub(crate) fps_input: Input,
    pub(crate) monitor_inputs: Vec<(Input, Input)>,
    api_key_source: WeatherApiKeySource,
    api_key_original_value: String,
}

impl FormState {
    pub(crate) fn new(config: &Config) -> Self {
        let city_search_input = Input::default().with_value(config.location.city.clone());
        let city_search_results = Vec::new();
        let city_search_selected_index = 0;
        let lat_input = Input::default().with_value(config.location.latitude.to_string());
        let lon_input = Input::default().with_value(config.location.longitude.to_string());
        let timezone_input = Input::default().with_value(config.location.timezone.clone());
        let (api_key_value, api_key_source) =
            resolved_weather_api_key_value(config, |env_name| std::env::var(env_name).ok());
        let api_key_input = Input::default().with_value(api_key_value.clone());

        let desktop_idle_timeout_minutes_input =
            Input::default().with_value(if config.daemon.desktop_idle_sync {
                config.daemon.desktop_idle_timeout_minutes.to_string()
            } else {
                "0".to_string()
            });
        let suspend_minutes_input = Input::default();
        let fps_input = Input::default().with_value(config.tui.fps.to_string());
        let monitor_inputs = config
            .monitors
            .iter()
            .map(|monitor| {
                (
                    Input::default().with_value(monitor.min_pct.to_string()),
                    Input::default().with_value(monitor.max_pct.to_string()),
                )
            })
            .collect::<Vec<_>>();

        Self {
            city_search_input,
            city_search_results,
            city_search_selected_index,
            lat_input,
            lon_input,
            timezone_input,
            api_key_input,

            desktop_idle_timeout_minutes_input,
            suspend_minutes_input,
            fps_input,
            monitor_inputs,
            api_key_source,
            api_key_original_value: api_key_value,
        }
    }

    pub(crate) fn automation_field_count(&self) -> usize {
        1 + self.monitor_inputs.len() * 2
    }

    pub(crate) fn active_input_kind(
        &self,
        active_tab: Tab,
        active_setting: usize,
    ) -> Option<ActiveInputKind> {
        match active_tab {
            Tab::Monitors => None,
            Tab::Limits => {
                let n = self.monitor_inputs.len() * 2;
                if active_setting == n {
                    Some(ActiveInputKind::Integer)
                } else {
                    Some(ActiveInputKind::Decimal)
                }
            }
            Tab::Location => Some(match active_setting {
                0 => ActiveInputKind::Text,
                1 | 2 => ActiveInputKind::Decimal,
                3 => ActiveInputKind::Text,
                _ => return None,
            }),
            Tab::Weather if active_setting == 0 => Some(ActiveInputKind::Secret),
            Tab::Settings => match active_setting {
                0 => Some(ActiveInputKind::Toggle), // Theme Toggle
                1 => Some(ActiveInputKind::Integer),
                2 => Some(ActiveInputKind::Toggle), // 12h Toggle
                3 => Some(ActiveInputKind::Toggle), // Unit Toggle
                4 => Some(ActiveInputKind::Integer),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn active_input_ref(
        &self,
        active_tab: Tab,
        active_setting: usize,
    ) -> Option<&Input> {
        match active_tab {
            Tab::Monitors => None,
            Tab::Limits => self.automation_input_ref(active_setting),
            Tab::Location => match active_setting {
                0 => Some(&self.city_search_input),
                1 => Some(&self.lat_input),
                2 => Some(&self.lon_input),
                _ => None,
            },
            Tab::Weather if active_setting == 0 => Some(&self.api_key_input),
            Tab::Settings => match active_setting {
                0 => None, // Theme Toggle
                1 => Some(&self.fps_input),
                2 => None, // Toggle
                3 => None, // Toggle
                4 => Some(&self.suspend_minutes_input),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn active_input_mut(
        &mut self,
        active_tab: Tab,
        active_setting: usize,
    ) -> Option<&mut Input> {
        match active_tab {
            Tab::Monitors => None,
            Tab::Limits => self.automation_input_mut(active_setting),
            Tab::Location => match active_setting {
                0 => Some(&mut self.city_search_input),
                1 => Some(&mut self.lat_input),
                2 => Some(&mut self.lon_input),
                _ => None,
            },
            Tab::Weather if active_setting == 0 => Some(&mut self.api_key_input),
            Tab::Settings => match active_setting {
                0 => None, // Theme Toggle
                1 => Some(&mut self.fps_input),
                2 => None, // Toggle
                3 => None, // Toggle
                4 => Some(&mut self.suspend_minutes_input),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn apply_to_config(&self, config: &mut Config) {
        let monitor_bounds = self
            .monitor_inputs
            .iter()
            .map(|(min_input, max_input)| {
                (min_input.value().to_string(), max_input.value().to_string())
            })
            .collect::<Vec<_>>();

        apply_form_values_to_config(
            config,
            self.city_search_input.value(),
            self.lat_input.value(),
            self.lon_input.value(),
            self.timezone_input.value(),
            self.desktop_idle_timeout_minutes_input.value(),
            self.fps_input.value(),
            &monitor_bounds,
        );
        apply_weather_api_key_to_config(
            config,
            self.api_key_input.value(),
            self.api_key_source,
            &self.api_key_original_value,
        );
    }

    pub(crate) fn refresh_from_config(&mut self, config: &Config) {
        let (api_key_value, api_key_source) =
            resolved_weather_api_key_value(config, |env_name| std::env::var(env_name).ok());
        self.api_key_input = Input::default().with_value(api_key_value.clone());
        self.api_key_source = api_key_source;
        self.api_key_original_value = api_key_value;
    }

    pub(crate) fn suspend_duration_minutes(&self) -> Result<Option<u64>, String> {
        parse_suspend_duration_minutes(self.suspend_minutes_input.value())
    }

    fn automation_input_ref(&self, index: usize) -> Option<&Input> {
        let monitor_field_count = self.monitor_inputs.len() * 2;
        if index < monitor_field_count {
            let monitor_index = index / 2;
            let is_max = index % 2 == 1;
            self.monitor_inputs
                .get(monitor_index)
                .map(|pair| if is_max { &pair.1 } else { &pair.0 })
        } else if index == monitor_field_count {
            Some(&self.desktop_idle_timeout_minutes_input)
        } else {
            None
        }
    }

    fn automation_input_mut(&mut self, index: usize) -> Option<&mut Input> {
        let monitor_field_count = self.monitor_inputs.len() * 2;
        if index < monitor_field_count {
            let monitor_index = index / 2;
            let is_max = index % 2 == 1;
            self.monitor_inputs.get_mut(monitor_index).map(|pair| {
                if is_max {
                    &mut pair.1
                } else {
                    &mut pair.0
                }
            })
        } else if index == monitor_field_count {
            Some(&mut self.desktop_idle_timeout_minutes_input)
        } else {
            None
        }
    }
}

fn parse_suspend_duration_minutes(raw: &str) -> Result<Option<u64>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let minutes = trimmed
        .parse::<u64>()
        .map_err(|_| String::from("suspend duration must be a whole number of minutes"))?;
    if minutes == 0 {
        return Err(String::from("suspend duration must be greater than zero"));
    }

    Ok(Some(minutes))
}

fn apply_form_values_to_config(
    config: &mut Config,
    city: &str,
    latitude: &str,
    longitude: &str,
    timezone: &str,
    desktop_idle_timeout_minutes: &str,
    fps: &str,
    monitor_bounds: &[(String, String)],
) {
    config.location.city = city.trim().to_string();
    if let Ok(lat) = latitude.trim().parse::<f64>() {
        config.location.latitude = lat;
    }
    if let Ok(lon) = longitude.trim().parse::<f64>() {
        config.location.longitude = lon;
    }
    let tz = timezone.trim();
    if !tz.is_empty() {
        config.location.timezone = tz.to_string();
    }
    if let Ok(minutes) = desktop_idle_timeout_minutes.trim().parse::<u64>() {
        if minutes == 0 {
            config.daemon.desktop_idle_sync = false;
        } else {
            config.daemon.desktop_idle_sync = true;
            config.daemon.desktop_idle_timeout_minutes = minutes;
        }
    }
    if let Ok(fps_val) = fps.trim().parse::<u32>() {
        if fps_val > 0 {
            config.tui.fps = fps_val;
        }
    }
    for (index, bounds) in monitor_bounds.iter().enumerate() {
        if let Some(monitor) = config.monitors.get_mut(index) {
            if let Ok(min) = bounds.0.trim().parse::<u8>() {
                monitor.min_pct = min;
            }
            if let Ok(max) = bounds.1.trim().parse::<u8>() {
                monitor.max_pct = max;
            }
        }
    }
}

fn resolved_weather_api_key_value<E>(config: &Config, read_env: E) -> (String, WeatherApiKeySource)
where
    E: Fn(&str) -> Option<String>,
{
    if let Some(api_key) = config
        .weather
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|api_key| !api_key.is_empty())
    {
        return (api_key.to_owned(), WeatherApiKeySource::Config);
    }

    if let Some(env_name) = config
        .weather
        .api_key_env
        .as_deref()
        .map(str::trim)
        .filter(|env_name| !env_name.is_empty())
    {
        if let Some(api_key) = read_env(env_name)
            .as_deref()
            .map(str::trim)
            .filter(|api_key| !api_key.is_empty())
        {
            return (api_key.to_owned(), WeatherApiKeySource::Environment);
        }
    }

    (String::new(), WeatherApiKeySource::Missing)
}

fn apply_weather_api_key_to_config(
    config: &mut Config,
    api_key: &str,
    source: WeatherApiKeySource,
    original_value: &str,
) {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        config.weather.api_key = None;
        return;
    }

    if source == WeatherApiKeySource::Environment && api_key == original_value.trim() {
        config.weather.api_key = None;
        return;
    }

    config.weather.api_key = Some(api_key.to_owned());
    config.weather.enabled = true;

    if source == WeatherApiKeySource::Environment {
        config.weather.api_key_env = None;
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, WeatherProvider};

    use super::{
        apply_form_values_to_config, apply_weather_api_key_to_config,
        parse_suspend_duration_minutes, resolved_weather_api_key_value, WeatherApiKeySource,
    };

    #[test]
    fn empty_weather_field_does_not_disable_env_backed_weather() {
        let mut config = Config::default();
        config.weather.enabled = true;
        config.weather.provider = Some(WeatherProvider::OpenWeather);
        config.weather.api_key_env = Some(String::from("OPENWEATHER_API_KEY"));

        apply_form_values_to_config(
            &mut config,
            "Istanbul",
            "41.0",
            "29.0",
            "Europe/Istanbul",
            "5",
            "8",
            &[],
        );
        apply_weather_api_key_to_config(&mut config, "", WeatherApiKeySource::Missing, "");

        assert!(config.weather.enabled);
        assert_eq!(
            config.weather.api_key_env.as_deref(),
            Some("OPENWEATHER_API_KEY")
        );
        assert_eq!(config.weather.api_key, None);
    }

    #[test]
    fn explicit_weather_api_key_enables_weather() {
        let mut config = Config::default();
        config.weather.enabled = false;
        config.weather.api_key_env = None;

        apply_form_values_to_config(
            &mut config,
            "Istanbul",
            "41.0",
            "29.0",
            "Europe/Istanbul",
            "5",
            "8",
            &[],
        );
        apply_weather_api_key_to_config(&mut config, "secret", WeatherApiKeySource::Missing, "");

        assert!(config.weather.enabled);
        assert_eq!(config.weather.api_key.as_deref(), Some("secret"));
    }

    #[test]
    fn form_updates_timezone() {
        let mut config = Config::default();
        config.solar_policy.twilight_elevation_start = -8.0;
        config.solar_policy.day_elevation_full = 15.0;

        apply_form_values_to_config(
            &mut config,
            "Istanbul",
            "41.0",
            "29.0",
            "Europe/Istanbul",
            "5",
            "8",
            &[],
        );

        assert_eq!(config.location.timezone, "Europe/Istanbul");
        assert_eq!(config.solar_policy.twilight_elevation_start, -8.0);
        assert_eq!(config.solar_policy.day_elevation_full, 15.0);
    }

    #[test]
    fn env_backed_weather_api_key_is_loaded_for_mask_and_reveal() {
        let mut config = Config::default();
        config.weather.api_key = None;
        config.weather.api_key_env = Some(String::from("SUNREACTOR_TEST_KEY"));

        let (api_key, source) = resolved_weather_api_key_value(&config, |env_name| {
            if env_name == "SUNREACTOR_TEST_KEY" {
                Some(String::from("super-secret-key"))
            } else {
                None
            }
        });

        assert_eq!(api_key, "super-secret-key");
        assert_eq!(source, WeatherApiKeySource::Environment);
    }

    #[test]
    fn unchanged_env_backed_weather_api_key_is_not_written_inline() {
        let mut config = Config::default();
        config.weather.enabled = true;
        config.weather.api_key = None;
        config.weather.api_key_env = Some(String::from("OPENWEATHER_API_KEY"));

        apply_weather_api_key_to_config(
            &mut config,
            "super-secret-key",
            WeatherApiKeySource::Environment,
            "super-secret-key",
        );

        assert_eq!(config.weather.api_key, None);
        assert_eq!(
            config.weather.api_key_env.as_deref(),
            Some("OPENWEATHER_API_KEY")
        );
    }

    #[test]
    fn edited_env_backed_weather_api_key_switches_to_explicit_key() {
        let mut config = Config::default();
        config.weather.enabled = true;
        config.weather.api_key = None;
        config.weather.api_key_env = Some(String::from("OPENWEATHER_API_KEY"));

        apply_weather_api_key_to_config(
            &mut config,
            "new-inline-key",
            WeatherApiKeySource::Environment,
            "old-env-key",
        );

        assert_eq!(config.weather.api_key.as_deref(), Some("new-inline-key"));
        assert_eq!(config.weather.api_key_env, None);
    }

    #[test]
    fn suspend_duration_parser_accepts_positive_integer() {
        assert_eq!(parse_suspend_duration_minutes("45"), Ok(Some(45)));
    }

    #[test]
    fn suspend_duration_parser_allows_blank_for_indefinite() {
        assert_eq!(parse_suspend_duration_minutes(""), Ok(None));
    }

    #[test]
    fn suspend_duration_parser_rejects_zero() {
        assert_eq!(
            parse_suspend_duration_minutes("0"),
            Err(String::from("suspend duration must be greater than zero"))
        );
    }
}
