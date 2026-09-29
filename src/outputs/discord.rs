//! Discord Rich Presence via discord-rich-presence.
//! Never uses Kopuz's application id. Music mode defaults to coexist.
//!
//! Art assets (`large_image` / `small_image`) are Discord Application asset keys
//! (or HTTPS URLs — both are accepted by the crate). Upload keys in the Developer
//! Portal → Rich Presence → Art Assets for application id configured in
//! `[discord].application_id`.
use crate::config::load::effective_discord_app_id;
use crate::config::{Config, DiscordMusicMode};
use crate::providers::State;
use crate::util::find_discord_ipc_sockets;
use anyhow::{bail, Context, Result};
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use tracing::{debug, info, warn};

const FORBIDDEN_KOPUZ_APP_ID: &str = "1470087339639443658";

#[derive(Default)]
pub struct DiscordOutput {
    client: Option<DiscordIpcClient>,
    connected: bool,
    last_details: String,
    last_state: String,
    last_large: String,
    last_large_text: String,
    last_small: String,
    last_small_text: String,
}

impl DiscordOutput {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn ipc_sockets_present() -> bool {
        !find_discord_ipc_sockets().is_empty()
    }

    fn resolve_app_id(cfg: &Config) -> Result<String> {
        let id = effective_discord_app_id(cfg).context("discord application_id not set")?;
        if id == FORBIDDEN_KOPUZ_APP_ID {
            bail!("refusing Kopuz Discord application id {FORBIDDEN_KOPUZ_APP_ID}");
        }
        Ok(id)
    }

    pub fn connect(&mut self, cfg: &Config) -> Result<()> {
        if !cfg.discord.enabled {
            return Ok(());
        }
        let id = Self::resolve_app_id(cfg)?;
        if find_discord_ipc_sockets().is_empty() {
            bail!("no discord-ipc-* under XDG_RUNTIME_DIR (is Equibop/Discord running?)");
        }
        let mut client = DiscordIpcClient::new(&id).map_err(|e| anyhow::anyhow!("{e}"))?;
        client
            .connect()
            .map_err(|e| anyhow::anyhow!("Discord IPC connect: {e}"))?;
        self.client = Some(client);
        self.connected = true;
        info!("Discord Rich Presence connected (app id set)");
        Ok(())
    }

    fn preferred_player_active(&self, cfg: &Config, state: &State) -> bool {
        let Some(player) = &state.player else {
            return false;
        };
        cfg.discord.music.prefer_players.iter().any(|p| {
            p.eq_ignore_ascii_case(player) || player.to_lowercase().contains(&p.to_lowercase())
        })
    }

    pub fn publish(&mut self, cfg: &Config, state: &State, composed: &str) -> Result<()> {
        if !cfg.discord.enabled {
            return Ok(());
        }
        if !self.connected {
            if let Err(e) = self.connect(cfg) {
                debug!("Discord connect deferred: {e}");
                return Ok(());
            }
        }

        let music_active = state.artist.is_some() || state.title.is_some();
        let prefer = self.preferred_player_active(cfg, state);
        match cfg.discord.music.mode {
            DiscordMusicMode::Coexist if prefer && music_active => {
                debug!("discord.music=coexist; preferred player owns music RPC");
            }
            DiscordMusicMode::Defer if prefer => {
                debug!("discord.music=defer; skipping Discord update");
                return Ok(());
            }
            _ => {}
        }

        let details = render_simple(&cfg.discord.details_template, state, composed, cfg);
        let state_s = render_simple(&cfg.discord.state_template, state, composed, cfg);

        let (details, state_s) = if matches!(cfg.discord.music.mode, DiscordMusicMode::Coexist)
            && prefer
            && music_active
        {
            let d = if looks_like_music(&details, state) {
                composed_without_music(composed, state).unwrap_or_else(|| state.custom_text(cfg))
            } else {
                details
            };
            let s = if looks_like_music(&state_s, state) {
                String::new()
            } else {
                state_s
            };
            (d, s)
        } else {
            (details, state_s)
        };

        let large = cfg.discord.assets.large_image.trim().to_string();
        let large_text = cfg.discord.assets.large_text.trim().to_string();
        let (small, small_text) = resolve_small_asset(cfg, state, prefer);

        if details == self.last_details
            && state_s == self.last_state
            && large == self.last_large
            && large_text == self.last_large_text
            && small == self.last_small
            && small_text == self.last_small_text
        {
            return Ok(());
        }

        let Some(client) = self.client.as_mut() else {
            return Ok(());
        };

        let mut act = activity::Activity::new();
        if !details.is_empty() {
            act = act.details(&details);
        }
        if !state_s.is_empty() {
            act = act.state(&state_s);
        }

        if !large.is_empty() || !small.is_empty() {
            let mut assets = activity::Assets::new();
            if !large.is_empty() {
                assets = assets.large_image(&large);
            }
            if !large_text.is_empty() {
                assets = assets.large_text(&large_text);
            }
            if !small.is_empty() {
                assets = assets.small_image(&small);
            }
            if !small_text.is_empty() {
                assets = assets.small_text(&small_text);
            }
            act = act.assets(assets);
        }

        match client.set_activity(act) {
            Ok(()) => {
                self.last_details = details;
                self.last_state = state_s;
                self.last_large = large;
                self.last_large_text = large_text;
                self.last_small = small;
                self.last_small_text = small_text;
            }
            Err(e) => {
                warn!("Discord set_activity: {e}");
                self.connected = false;
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        if let Some(c) = self.client.as_mut() {
            let _ = c.clear_activity();
        }
    }
}

/// Resolve small-image asset key + tooltip.
///
/// Priority:
/// 1. `[discord.assets].small_image` override (if non-empty)
/// 2. VRChat running → `map.vrchat`
/// 3. Preferred music player active → `map.<player>` / `map.kopuz`
/// 4. `map.default` — or omit small image when unset / empty
fn resolve_small_asset(cfg: &Config, state: &State, prefer: bool) -> (String, String) {
    let assets = &cfg.discord.assets;

    if let Some(over) = &assets.small_image {
        let key = over.trim();
        if !key.is_empty() {
            let text = assets
                .small_text
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("")
                .to_string();
            return (key.to_string(), text);
        }
    }

    if state.vrchat_running {
        if let Some(key) = map_get(&assets.map, "vrchat") {
            let text = assets
                .small_text
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "VRChat".into());
            return (key, text);
        }
    }

    if prefer {
        if let Some(player) = &state.player {
            if let Some(key) = map_get_for_player(&assets.map, player) {
                let text = assets
                    .small_text
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| player.clone());
                return (key, text);
            }
        }
    }

    if let Some(key) = map_get(&assets.map, "default") {
        if !key.is_empty() {
            let text = assets
                .small_text
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "nixpresence".into());
            return (key, text);
        }
    }

    (String::new(), String::new())
}

fn map_get(map: &std::collections::HashMap<String, String>, logical: &str) -> Option<String> {
    if let Some(v) = map.get(logical) {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    map.iter()
        .find(|(k, v)| k.eq_ignore_ascii_case(logical) && !v.trim().is_empty())
        .map(|(_, v)| v.trim().to_string())
}

fn map_get_for_player(
    map: &std::collections::HashMap<String, String>,
    player: &str,
) -> Option<String> {
    let normalized = normalize_player_key(player);
    if let Some(k) = map_get(map, &normalized) {
        return Some(k);
    }
    // Prefer known aliases when prefer_players matched Kopuz etc.
    for alias in ["kopuz", "equibop", "spotify", "vlc"] {
        if player.to_lowercase().contains(alias) {
            if let Some(k) = map_get(map, alias) {
                return Some(k);
            }
        }
    }
    map_get(map, player)
}

fn normalize_player_key(player: &str) -> String {
    player
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn render_simple(tmpl: &str, state: &State, composed: &str, cfg: &Config) -> String {
    let mut s = tmpl.to_string();
    s = s.replace("{custom}", &state.custom_text(cfg));
    s = s.replace("{os}", state.os_pretty.as_deref().unwrap_or(""));
    s = s.replace("{gpu_name}", state.gpu_name.as_deref().unwrap_or(""));
    s = s.replace("{gpu}", state.gpu_name.as_deref().unwrap_or(""));
    s = s.replace("{music}", &state.music_display(cfg));
    s = s.replace("{composed}", composed);
    s = s.replace("{profile}", &state.active_profile);
    if s.chars().count() > 128 {
        s = s.chars().take(125).collect::<String>() + "…";
    }
    s
}

fn looks_like_music(text: &str, state: &State) -> bool {
    if text.is_empty() {
        return false;
    }
    if let Some(t) = &state.title {
        if !t.is_empty() && text.contains(t) {
            return true;
        }
    }
    if let Some(a) = &state.artist {
        if !a.is_empty() && text.contains(a) {
            return true;
        }
    }
    text.contains('🎵')
}

fn composed_without_music(composed: &str, state: &State) -> Option<String> {
    let mut s = composed.to_string();
    if let Some(t) = &state.title {
        s = s.replace(t, "");
    }
    if let Some(a) = &state.artist {
        s = s.replace(a, "");
    }
    s = s.replace('🎵', "");
    let s = s
        .trim()
        .trim_matches(|c| c == '┆' || c == '|' || c == '-')
        .trim()
        .to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DiscordAssetsConfig, DiscordConfig, DiscordMusicConfig};
    use std::collections::HashMap;

    fn cfg_with_assets() -> Config {
        Config {
            discord: DiscordConfig {
                enabled: true,
                application_id: "1554458551001550851".into(),
                assets: DiscordAssetsConfig::default(),
                music: DiscordMusicConfig::default(),
                ..DiscordConfig::default()
            },
            ..Config::default()
        }
    }

    #[test]
    fn small_prefers_vrchat_when_running() {
        let cfg = cfg_with_assets();
        let state = State {
            vrchat_running: true,
            player: Some("Kopuz".into()),
            ..State::default()
        };
        let (key, text) = resolve_small_asset(&cfg, &state, true);
        assert_eq!(key, "vrchat");
        assert_eq!(text, "VRChat");
    }

    #[test]
    fn small_uses_kopuz_when_preferred_player() {
        let cfg = cfg_with_assets();
        let state = State {
            player: Some("Kopuz".into()),
            ..State::default()
        };
        let (key, text) = resolve_small_asset(&cfg, &state, true);
        assert_eq!(key, "kopuz");
        assert_eq!(text, "Kopuz");
    }

    #[test]
    fn small_falls_back_to_default_map() {
        let cfg = cfg_with_assets();
        let state = State::default();
        let (key, text) = resolve_small_asset(&cfg, &state, false);
        assert_eq!(key, "nixpresence");
        assert_eq!(text, "nixpresence");
    }

    #[test]
    fn small_override_wins() {
        let mut cfg = cfg_with_assets();
        cfg.discord.assets.small_image = Some("equibop".into());
        cfg.discord.assets.small_text = Some("Equibop".into());
        let state = State {
            vrchat_running: true,
            ..State::default()
        };
        let (key, text) = resolve_small_asset(&cfg, &state, false);
        assert_eq!(key, "equibop");
        assert_eq!(text, "Equibop");
    }

    #[test]
    fn small_omitted_when_default_unset() {
        let mut cfg = cfg_with_assets();
        cfg.discord.assets.map = HashMap::new();
        let state = State::default();
        let (key, text) = resolve_small_asset(&cfg, &state, false);
        assert!(key.is_empty());
        assert!(text.is_empty());
    }

    #[test]
    fn normalize_player_key_strips_junk() {
        assert_eq!(normalize_player_key("Kopuz"), "kopuz");
        assert_eq!(
            normalize_player_key(" org.mpris.MediaPlayer2.kopuz "),
            "org_mpris_mediaplayer2_kopuz"
        );
    }
}
