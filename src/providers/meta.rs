//! Local time + configured location text.
use super::State;
use crate::config::Config;
use chrono::Utc;
use chrono_tz::Tz;

#[derive(Debug, Default)]
pub struct MetaProvider;

impl MetaProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh(&self, cfg: &Config, state: &mut State) {
        if cfg.modules.location && cfg.location.enabled {
            let t = cfg.location.text.trim();
            state.location = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        } else {
            state.location = None;
        }

        if cfg.modules.time && cfg.time.enabled {
            state.local_time = Some(format_local_time(&cfg.time.timezone, &cfg.time.format));
        } else {
            state.local_time = None;
        }
    }
}

pub fn format_local_time(tz_name: &str, fmt: &str) -> String {
    let tz: Tz = tz_name.parse().unwrap_or(chrono_tz::America::Los_Angeles);
    let now = Utc::now().with_timezone(&tz);
    now.format(fmt).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_la_time() {
        let s = format_local_time("America/Los_Angeles", "%-I:%M %p %Z");
        assert!(!s.is_empty(), "empty time");
        // PST or PDT depending on DST
        assert!(
            s.contains("PST") || s.contains("PDT") || s.contains("GMT") || s.contains("UTC"),
            "got {s}"
        );
    }
}
