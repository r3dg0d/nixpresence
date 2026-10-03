//! Open-Meteo weather (cached, privacy-safe — only configured coords).
use super::State;
use crate::config::Config;
use serde::Deserialize;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

#[derive(Debug, Default)]
pub struct WeatherProvider {
    last_fetch: Option<Instant>,
    cached_line: Option<String>,
    cached_temp: Option<String>,
    cached_condition: Option<String>,
}

impl WeatherProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh(&mut self, cfg: &Config, state: &mut State) {
        self.refresh_with(cfg, state, &UreqOpenMeteo);
    }

    fn refresh_with<C: OpenMeteoClient>(&mut self, cfg: &Config, state: &mut State, client: &C) {
        if !cfg.weather.enabled || !cfg.modules.weather {
            state.weather = None;
            state.weather_temp = None;
            state.weather_condition = None;
            return;
        }
        let lat = cfg.location.latitude;
        let lon = cfg.location.longitude;
        if lat == 0.0 && lon == 0.0 {
            debug!("weather enabled but location lat/lon unset");
            return;
        }

        let cache = Duration::from_secs_f64(cfg.weather.cache_secs.max(60.0));
        let fresh = self
            .last_fetch
            .map(|t| t.elapsed() < cache)
            .unwrap_or(false);
        if fresh {
            state.weather = self.cached_line.clone();
            state.weather_temp = self.cached_temp.clone();
            state.weather_condition = self.cached_condition.clone();
            return;
        }

        match fetch_open_meteo(client, lat, lon, &cfg.weather.units) {
            Ok((temp, condition, line)) => {
                self.cached_temp = Some(temp.clone());
                self.cached_condition = Some(condition.clone());
                self.cached_line = Some(line.clone());
                self.last_fetch = Some(Instant::now());
                state.weather_temp = Some(temp);
                state.weather_condition = Some(condition);
                state.weather = Some(line);
            }
            Err(e) => {
                warn!("weather fetch failed: {e}");
                // Same cache window as a success so a down API is not hit every tick.
                // Keep the last good sample if we already have one.
                self.last_fetch = Some(Instant::now());
                state.weather = self.cached_line.clone();
                state.weather_temp = self.cached_temp.clone();
                state.weather_condition = self.cached_condition.clone();
            }
        }
    }

    /// Pretend `cache_secs` has elapsed so the next refresh may hit the client again.
    #[cfg(test)]
    fn expire_backoff_for_test(&mut self) {
        self.last_fetch = None;
    }

    #[cfg(test)]
    fn last_fetch_is_set(&self) -> bool {
        self.last_fetch.is_some()
    }
}

/// HTTP seam for Open-Meteo. Production uses ureq; tests use a scripted client.
trait OpenMeteoClient {
    fn get_json(&self, url: &str) -> anyhow::Result<OpenMeteoResponse>;
}

struct UreqOpenMeteo;

impl OpenMeteoClient for UreqOpenMeteo {
    fn get_json(&self, url: &str) -> anyhow::Result<OpenMeteoResponse> {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(8))
            .user_agent("nixpresence/0.1 (+https://github.com/r3dg0d/nixpresence)")
            .build();
        Ok(agent.get(url).call()?.into_json()?)
    }
}

fn fetch_open_meteo<C: OpenMeteoClient>(
    client: &C,
    lat: f64,
    lon: f64,
    units: &str,
) -> anyhow::Result<(String, String, String)> {
    let temp_unit = if units.eq_ignore_ascii_case("celsius") || units.eq_ignore_ascii_case("c") {
        "celsius"
    } else {
        "fahrenheit"
    };
    let unit_suffix = if temp_unit == "celsius" { "°C" } else { "°F" };
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,weather_code&temperature_unit={temp_unit}&timezone=auto"
    );
    let resp = client.get_json(&url)?;
    let temp_f = resp.current.temperature_2m;
    let code = resp.current.weather_code;
    let condition = wmo_condition(code);
    let temp = format!("{temp_f:.0}{unit_suffix}");
    let line = format!("{temp} {condition}");
    Ok((temp, condition.to_string(), line))
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f64,
    weather_code: u32,
}

fn wmo_condition(code: u32) -> &'static str {
    match code {
        0 => "Clear",
        1..=2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55..=57 => "Drizzle",
        61 | 63 | 65..=67 => "Rain",
        71 | 73 | 75 | 77 => "Snow",
        80..=82 => "Showers",
        85..=86 => "Snow showers",
        95 | 96 | 99 => "Thunderstorm",
        _ => "Weather",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    fn weather_cfg() -> Config {
        let mut cfg = Config::default();
        cfg.modules.weather = true;
        cfg.weather.enabled = true;
        cfg.weather.cache_secs = 600.0;
        cfg.weather.units = "fahrenheit".into();
        cfg.location.latitude = 33.72;
        cfg.location.longitude = -116.37;
        cfg
    }

    fn sample(temp: f64, code: u32) -> OpenMeteoResponse {
        OpenMeteoResponse {
            current: OpenMeteoCurrent {
                temperature_2m: temp,
                weather_code: code,
            },
        }
    }

    struct FakeOpenMeteo {
        hits: Cell<usize>,
        outcomes: RefCell<VecDeque<Result<OpenMeteoResponse, String>>>,
    }

    impl FakeOpenMeteo {
        fn script(items: Vec<Result<OpenMeteoResponse, String>>) -> Self {
            Self {
                hits: Cell::new(0),
                outcomes: RefCell::new(items.into()),
            }
        }
    }

    impl OpenMeteoClient for FakeOpenMeteo {
        fn get_json(&self, url: &str) -> anyhow::Result<OpenMeteoResponse> {
            assert!(
                url.starts_with("https://api.open-meteo.com/v1/forecast?"),
                "{url}"
            );
            assert!(url.contains("latitude=33.72"), "{url}");
            assert!(url.contains("longitude=-116.37"), "{url}");
            self.hits.set(self.hits.get() + 1);
            match self.outcomes.borrow_mut().pop_front() {
                Some(Ok(body)) => Ok(body),
                Some(Err(msg)) => anyhow::bail!("{msg}"),
                None => panic!("Open-Meteo client called more often than scripted"),
            }
        }
    }

    #[test]
    fn wmo_clear() {
        assert_eq!(wmo_condition(0), "Clear");
        assert_eq!(wmo_condition(61), "Rain");
    }

    #[test]
    fn success_sets_last_fetch_and_skips_network_inside_cache() {
        let cfg = weather_cfg();
        let mut provider = WeatherProvider::new();
        let mut state = State::default();
        let client = FakeOpenMeteo::script(vec![Ok(sample(72.4, 0))]);

        provider.refresh_with(&cfg, &mut state, &client);
        assert!(provider.last_fetch_is_set());
        assert_eq!(state.weather.as_deref(), Some("72°F Clear"));
        assert_eq!(state.weather_temp.as_deref(), Some("72°F"));
        assert_eq!(state.weather_condition.as_deref(), Some("Clear"));
        assert_eq!(client.hits.get(), 1);

        provider.refresh_with(&cfg, &mut state, &client);
        assert_eq!(client.hits.get(), 1);
        assert_eq!(state.weather.as_deref(), Some("72°F Clear"));
    }

    #[test]
    fn failure_records_fetch_time_and_keeps_last_good_weather() {
        let cfg = weather_cfg();
        let mut provider = WeatherProvider::new();
        let mut state = State::default();
        let client =
            FakeOpenMeteo::script(vec![Ok(sample(72.4, 0)), Err("open-meteo down".into())]);

        provider.refresh_with(&cfg, &mut state, &client);
        assert_eq!(client.hits.get(), 1);
        assert_eq!(state.weather.as_deref(), Some("72°F Clear"));

        provider.expire_backoff_for_test();
        provider.refresh_with(&cfg, &mut state, &client);
        assert!(provider.last_fetch_is_set());
        assert_eq!(client.hits.get(), 2);
        assert_eq!(state.weather.as_deref(), Some("72°F Clear"));
        assert_eq!(state.weather_temp.as_deref(), Some("72°F"));
        assert_eq!(state.weather_condition.as_deref(), Some("Clear"));

        // Still inside cache_secs: do not hit the network again.
        provider.refresh_with(&cfg, &mut state, &client);
        assert_eq!(client.hits.get(), 2);
        assert_eq!(state.weather.as_deref(), Some("72°F Clear"));
    }

    #[test]
    fn failure_without_prior_sample_still_backs_off() {
        let cfg = weather_cfg();
        let mut provider = WeatherProvider::new();
        let mut state = State::default();
        let client = FakeOpenMeteo::script(vec![Err("timeout".into())]);

        provider.refresh_with(&cfg, &mut state, &client);
        assert!(provider.last_fetch_is_set());
        assert!(state.weather.is_none());
        assert!(state.weather_temp.is_none());
        assert!(state.weather_condition.is_none());
        assert_eq!(client.hits.get(), 1);

        provider.refresh_with(&cfg, &mut state, &client);
        assert_eq!(client.hits.get(), 1);
        assert!(state.weather.is_none());
    }
}
