use super::State;
use crate::config::Config;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct CustomProvider;

impl CustomProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh(&self, cfg: &Config, state: &mut State) {
        if state.custom_override.is_some() {
            return;
        }
        if cfg.custom.rotate.is_empty() {
            return;
        }
        let interval = Duration::from_secs_f64(cfg.custom.rotate_secs.max(1.0));
        match state.custom_rotate_at {
            // First tick: show rotate[0] before advancing.
            None => {
                state.custom_rotate_at = Some(Instant::now());
            }
            Some(at) if at.elapsed() < interval => {}
            Some(_) => {
                state.custom_rotate_idx = state.custom_rotate_idx.wrapping_add(1);
                state.custom_rotate_at = Some(Instant::now());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rotate_cfg() -> Config {
        let mut cfg = Config::default();
        cfg.custom.message.clear();
        cfg.custom.rotate = vec!["first".into(), "second".into(), "third".into()];
        cfg.custom.rotate_secs = 30.0;
        cfg
    }

    #[test]
    fn first_refresh_shows_first_rotate_line() {
        let cfg = rotate_cfg();
        let provider = CustomProvider::new();
        let mut state = State::default();

        assert_eq!(state.custom_text(&cfg), "first");
        provider.refresh(&cfg, &mut state);
        assert_eq!(state.custom_rotate_idx, 0);
        assert!(state.custom_rotate_at.is_some());
        assert_eq!(state.custom_text(&cfg), "first");

        // Still inside the interval: do not skip ahead.
        provider.refresh(&cfg, &mut state);
        assert_eq!(state.custom_rotate_idx, 0);
        assert_eq!(state.custom_text(&cfg), "first");
    }
}
