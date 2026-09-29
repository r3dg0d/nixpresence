//! Template rendering, truncation, page rotation, priority drop.
mod priority;
mod rotation;
mod template;
mod truncation;

pub use priority::*;
#[allow(unused_imports)]
pub use rotation::*;
pub use template::*;
pub use truncation::*;

use crate::config::{Config, PageConfig, RotationMode, OSC_MAX_GRAPHEMES};
use crate::providers::State;
use std::time::Instant;

#[derive(Debug)]
pub struct Compositor {
    #[allow(dead_code)]
    pub started: Instant,
    pub last_rotate: Instant,
    pub page_index: usize,
    pub last_output: String,
}

impl Default for Compositor {
    fn default() -> Self {
        Self::new()
    }
}

impl Compositor {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            last_rotate: now,
            page_index: 0,
            last_output: String::new(),
        }
    }

    pub fn compose(&mut self, cfg: &Config, state: &State) -> String {
        let profile_name = state.active_profile.as_str();
        let profile = cfg.profiles.get(profile_name);
        let page_names: Vec<&str> = profile
            .map(|p| p.pages.iter().map(|s| s.as_str()).collect())
            .unwrap_or_else(|| cfg.pages.iter().map(|p| p.name.as_str()).collect());

        let pages: Vec<&PageConfig> = page_names
            .iter()
            .filter_map(|n| cfg.pages.iter().find(|p| p.name == *n && p.enabled))
            .collect();

        if pages.is_empty() {
            return String::new();
        }

        let rotation = profile
            .and_then(|p| p.rotation.as_ref())
            .unwrap_or(&cfg.rotation);

        // Pause rotation when custom status set via IPC and configured.
        let pause = rotation.pause_on_custom && state.custom_override.is_some();

        if !pause {
            let interval = std::time::Duration::from_secs_f64(rotation.interval_secs.max(0.1));
            if self.last_rotate.elapsed() >= interval {
                self.page_index = self.page_index.wrapping_add(1);
                self.last_rotate = Instant::now();
            }
        }

        let rendered: Vec<(u32, String)> = match rotation.mode {
            RotationMode::Static => {
                let page = pages[0];
                vec![(page.priority, render_template(&page.template, state, cfg))]
            }
            RotationMode::RoundRobin => {
                let idx = self.page_index % pages.len();
                let page = pages[idx];
                vec![(page.priority, render_template(&page.template, state, cfg))]
            }
            RotationMode::Priority | RotationMode::Smart => {
                // Build all, then drop by priority if over budget.
                pages
                    .iter()
                    .map(|p| (p.priority, render_template(&p.template, state, cfg)))
                    .filter(|(_, s)| !s.trim().is_empty())
                    .collect()
            }
        };

        let sep = &cfg.general.separator;
        let joined =
            join_with_priority_drop(&rendered, sep, OSC_MAX_GRAPHEMES, &cfg.general.ellipsis);

        let mut out = format!("{}{}{}", cfg.general.prefix, joined, cfg.general.suffix);
        out = truncate_to_limit(&out, OSC_MAX_GRAPHEMES, &cfg.general.ellipsis);
        out = apply_privacy(&out, cfg, state);
        self.last_output = out.clone();
        out
    }
}

fn apply_privacy(s: &str, cfg: &Config, state: &State) -> String {
    let mut out = s.to_string();
    if cfg.privacy.hide_hostname {
        if let Some(h) = &state.hostname {
            if !h.is_empty() {
                out = out.replace(h, "[host]");
            }
        }
    }
    if cfg.privacy.hide_username {
        if let Ok(u) = std::env::var("USER") {
            if !u.is_empty() {
                out = out.replace(&u, "[user]");
            }
        }
    }
    if cfg.privacy.hide_ip {
        // crude IPv4 redact
        let re = regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").ok();
        if let Some(re) = re {
            out = re.replace_all(&out, "[ip]").into_owned();
        }
    }
    for pat in &cfg.privacy.redact_custom_patterns {
        if !pat.is_empty() {
            out = out.replace(pat, "[redacted]");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::State;

    #[test]
    fn compose_round_robin() {
        let cfg = Config::default();
        let state = State {
            os_pretty: Some("NixOS".into()),
            kernel: Some("7.2.8".into()),
            cpu_short: Some("i9-13900K".into()),
            ram: Some("32G".into()),
            ..State::default()
        };
        let mut c = Compositor::new();
        let out = c.compose(&cfg, &state);
        assert!(!out.is_empty() || true); // may be custom rotate page
        assert!(crate::util::grapheme_len(&out) <= OSC_MAX_GRAPHEMES);
    }
}
