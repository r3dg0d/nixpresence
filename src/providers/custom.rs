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
            Some(at) if at.elapsed() < interval => {}
            _ => {
                state.custom_rotate_idx = state.custom_rotate_idx.wrapping_add(1);
                state.custom_rotate_at = Some(Instant::now());
            }
        }
    }
}
