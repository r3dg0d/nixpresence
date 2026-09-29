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

        match fetch_open_meteo(lat, lon, &cfg.weather.units) {
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
                // Keep stale cache if any
                state.weather = self.cached_line.clone();
                state.weather_temp = self.cached_temp.clone();
                state.weather_condition = self.cached_condition.clone();
            }
        }
    }
}

fn fetch_open_meteo(lat: f64, lon: f64, units: &str) -> anyhow::Result<(String, String, String)> {
    let temp_unit = if units.eq_ignore_ascii_case("celsius") || units.eq_ignore_ascii_case("c") {
        "celsius"
    } else {
        "fahrenheit"
    };
    let unit_suffix = if temp_unit == "celsius" { "°C" } else { "°F" };
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,weather_code&temperature_unit={temp_unit}&timezone=auto"
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .user_agent("nixpresence/0.1 (+https://github.com/r3dg0d/nixpresence)")
        .build();
    let resp: OpenMeteoResponse = agent.get(&url).call()?.into_json()?;
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

    #[test]
    fn wmo_clear() {
        assert_eq!(wmo_condition(0), "Clear");
        assert_eq!(wmo_condition(61), "Rain");
    }
}
