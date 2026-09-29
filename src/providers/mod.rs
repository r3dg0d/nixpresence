//! Data providers → shared State.
pub mod custom;
pub mod focused;
pub mod hardware;
pub mod kernel;
pub mod kopuz;
pub mod mpris;
pub mod network;
pub mod nixos;
pub mod system;
pub mod vrchat;

use crate::config::Config;
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, Default)]
pub struct State {
    pub os_pretty: Option<String>,
    pub os_id: Option<String>,
    pub nixos_version: Option<String>,
    pub kernel: Option<String>,
    pub cpu: Option<String>,
    pub cpu_short: Option<String>,
    pub ram: Option<String>,
    pub ram_used: Option<String>,
    pub ram_total: Option<String>,
    pub gpu_name: Option<String>,
    pub gpu_temp_c: Option<f64>,
    pub gpu_util_pct: Option<u32>,
    pub gpu_mem: Option<String>,
    pub hostname: Option<String>,
    pub uptime: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub player: Option<String>,
    /// MPRIS `mpris:artUrl` (https / http / file).
    pub art_url: Option<String>,
    pub music_playing: bool,
    /// Focused window class (Hyprland / X11), e.g. `firefox`, `steam`.
    pub focused_app: Option<String>,
    pub focused_title: Option<String>,
    pub custom_override: Option<String>,
    pub custom_rotate_idx: usize,
    pub custom_rotate_at: Option<Instant>,
    pub vrchat_running: bool,
    pub vrchat_via: Vec<String>,
    pub active_profile: String,
    pub network_iface: Option<String>,
    pub last_tick: Option<SystemTime>,
    pub kopuz_socket_present: bool,
}

impl State {
    pub fn music_display(&self, cfg: &Config) -> String {
        if !cfg.music.enabled {
            return String::new();
        }
        let artist = self.artist.as_deref().unwrap_or("");
        let title = self.title.as_deref().unwrap_or("");
        if artist.is_empty() && title.is_empty() {
            return String::new();
        }
        let mut out = cfg.music.template.clone();
        out = out.replace("{artist}", artist);
        out = out.replace("{title}", title);
        out = out.replace("{album}", self.album.as_deref().unwrap_or(""));
        out = out.replace("{player}", self.player.as_deref().unwrap_or(""));
        out.trim()
            .trim_matches(|c| c == '-' || c == ' ')
            .to_string()
    }

    pub fn custom_text(&self, cfg: &Config) -> String {
        if let Some(o) = &self.custom_override {
            return o.clone();
        }
        if !cfg.custom.message.is_empty() {
            return cfg.custom.message.clone();
        }
        if cfg.custom.rotate.is_empty() {
            return String::new();
        }
        let idx = self.custom_rotate_idx % cfg.custom.rotate.len();
        cfg.custom.rotate[idx].clone()
    }
}

pub struct ProviderHub {
    pub system: system::SystemProvider,
    pub hardware: hardware::HardwareProvider,
    pub nixos: nixos::NixosProvider,
    pub kernel: kernel::KernelProvider,
    pub network: network::NetworkProvider,
    pub mpris: mpris::MprisProvider,
    pub kopuz: kopuz::KopuzProvider,
    pub vrchat: vrchat::VrchatProvider,
    pub focused: focused::FocusedAppProvider,
    pub custom: custom::CustomProvider,
}

impl ProviderHub {
    pub fn new(cfg: &Config) -> Self {
        Self {
            system: system::SystemProvider::new(),
            hardware: hardware::HardwareProvider::new(cfg),
            nixos: nixos::NixosProvider::new(),
            kernel: kernel::KernelProvider::new(),
            network: network::NetworkProvider::new(),
            mpris: mpris::MprisProvider::new(),
            kopuz: kopuz::KopuzProvider::new(cfg),
            vrchat: vrchat::VrchatProvider::new(),
            focused: focused::FocusedAppProvider::new(),
            custom: custom::CustomProvider::new(),
        }
    }

    pub async fn refresh(&mut self, cfg: &Config, state: &mut State) {
        if cfg.modules.system {
            self.system.refresh(state);
        }
        if cfg.modules.kernel {
            self.kernel.refresh(state);
        }
        if cfg.modules.nixos {
            self.nixos.refresh(state);
        }
        if cfg.modules.hardware {
            self.hardware.refresh(state);
        }
        if cfg.modules.network {
            self.network.refresh(cfg, state);
        }
        if cfg.modules.vrchat {
            self.vrchat.refresh(state);
        }
        if cfg.modules.focused_app {
            self.focused.refresh(state);
        }
        if cfg.modules.custom {
            self.custom.refresh(cfg, state);
        }

        // Music: MPRIS primary; Kopuz gRPC best-effort only.
        state.artist = None;
        state.title = None;
        state.album = None;
        state.player = None;
        state.art_url = None;
        state.music_playing = false;
        state.kopuz_socket_present = self.kopuz.socket_present();
        let mut got_music = false;
        if cfg.modules.kopuz && cfg.music.prefer_kopuz {
            if let Some(()) = self.kopuz.try_refresh(state).await {
                got_music = state.artist.is_some() || state.title.is_some();
            }
        }
        if cfg.modules.mpris && cfg.music.mpris && !got_music {
            let _ = self.mpris.refresh(cfg, state).await;
        } else if cfg.modules.mpris && cfg.music.mpris && cfg.music.prefer_kopuz {
            // Still prefer MPRIS player named Kopuz if gRPC empty
            if !got_music {
                let _ = self.mpris.refresh(cfg, state).await;
            }
        }

        // Auto profile switch
        if cfg.auto_profile.enabled {
            if state.vrchat_running {
                state.active_profile = cfg.auto_profile.on_vrchat.clone();
            } else if state.active_profile == cfg.auto_profile.on_vrchat
                || state.active_profile.is_empty()
            {
                state.active_profile = cfg.auto_profile.on_idle.clone();
            }
        } else if state.active_profile.is_empty() {
            state.active_profile = cfg.general.active_profile.clone();
        }

        state.last_tick = Some(SystemTime::now());
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.1}G", b / GIB)
    } else if b >= MIB {
        format!("{:.0}M", b / MIB)
    } else if b >= KIB {
        format!("{:.0}K", b / KIB)
    } else {
        format!("{bytes}B")
    }
}

pub fn format_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{d}d{h}h")
    } else if h > 0 {
        format!("{h}h{m}m")
    } else {
        format!("{m}m")
    }
}

#[allow(dead_code)]
pub fn ago(instant: Instant) -> Duration {
    instant.elapsed()
}
