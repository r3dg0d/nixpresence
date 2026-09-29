use super::State;
use crate::config::Config;

#[derive(Debug, Default)]
pub struct NetworkProvider;

impl NetworkProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh(&self, cfg: &Config, state: &mut State) {
        if cfg.privacy.hide_ip && cfg.privacy.hide_ssid {
            // module may be on but privacy strips; still record iface name only
        }
        // Minimal: default route iface from /proc/net/route
        if let Ok(text) = std::fs::read_to_string("/proc/net/route") {
            for line in text.lines().skip(1) {
                let cols: Vec<_> = line.split_whitespace().collect();
                if cols.len() >= 2 && cols[1] == "00000000" {
                    state.network_iface = Some(cols[0].to_string());
                    break;
                }
            }
        }
    }
}
