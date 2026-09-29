//! Discord Rich Presence via discord-rich-presence.
//! Never uses Kopuz's application id. Music mode defaults to coexist.
//!
//! Art assets (`large_image` / `small_image`) accept Discord Application asset
//! keys **or** HTTPS URLs. Prefer HTTPS when `prefer_https` is set and a value
//! looks like `https://`. Localhost / `file://` art is skipped by default
//! (Discord's CDN generally cannot fetch loopback).
//! Upload keys in Developer Portal → Rich Presence → Art Assets for the
//! application id in `[discord].application_id`.
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
        // Coexist + preferred player: leave music RP to Kopuz (etc.), but still
        // publish non-music presence including focused-app small_image.
        let defer_music_rp =
            matches!(cfg.discord.music.mode, DiscordMusicMode::Coexist) && prefer && music_active;

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

        let (details, state_s) = if defer_music_rp {
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

        let large = resolve_large_asset(cfg);
        let large_text = cfg.discord.assets.large_text.trim().to_string();
        let (small, small_text) = resolve_small_asset(cfg, state, prefer, defer_music_rp);

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

/// Large image: configured HTTPS URL (preferred) or portal asset key.
fn resolve_large_asset(cfg: &Config) -> String {
    let raw = cfg.discord.assets.large_image.trim();
    if raw.is_empty() {
        return String::new();
    }
    if let Some(url) = normalize_image_ref(raw, cfg.discord.assets.prefer_https, false) {
        return url;
    }
    raw.to_string()
}

/// Resolve small-image asset (HTTPS URL or portal key) + tooltip.
///
/// Priority:
/// 1. `[discord.assets].small_image` override (if non-empty)
/// 2. MPRIS `art_url` when https (or safe http→https) and music RP is ours
///    (skipped in coexist-defer-to-Kopuz path — focused app still shown)
/// 3. Focused window class → `map` / built-in aliases (HTTPS preferred)
/// 4. VRChat running → `map.vrchat`
/// 5. Preferred music player → `map.<player>` / `map.kopuz`
/// 6. `map.default` — or omit when unset / empty
fn resolve_small_asset(
    cfg: &Config,
    state: &State,
    prefer: bool,
    defer_music_rp: bool,
) -> (String, String) {
    let assets = &cfg.discord.assets;
    let prefer_https = assets.prefer_https;
    let serve_local = assets.serve_local_art;

    if let Some(over) = &assets.small_image {
        let key = over.trim();
        if !key.is_empty() {
            let resolved = normalize_image_ref(key, prefer_https, serve_local)
                .unwrap_or_else(|| key.to_string());
            let text = assets
                .small_text
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("")
                .to_string();
            return (resolved, text);
        }
    }

    // Album art when we own music presence (not coexist→Kopuz).
    if !defer_music_rp {
        if let Some(art) = state.art_url.as_deref() {
            if let Some(url) = normalize_image_ref(art, prefer_https, serve_local) {
                let text = assets
                    .small_text
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| {
                        state
                            .title
                            .clone()
                            .or_else(|| state.player.clone())
                            .unwrap_or_else(|| "Now playing".into())
                    });
                return (url, text);
            }
        }
    }

    // Focused app (always eligible, including coexist).
    if let Some(class) = state.focused_app.as_deref() {
        if let Some(key) = resolve_app_asset(&assets.map, class, prefer_https) {
            let text = assets
                .small_text
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| {
                    state
                        .focused_title
                        .clone()
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| class.to_string())
                });
            // Truncate tooltip (Discord ~128)
            let text = truncate_tooltip(&text);
            return (key, text);
        }
    }

    if state.vrchat_running {
        if let Some(key) = map_get_resolved(&assets.map, "vrchat", prefer_https) {
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
            if let Some(key) = map_get_for_player(&assets.map, player, prefer_https) {
                let text = assets
                    .small_text
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| player.clone());
                return (key, text);
            }
        }
    }

    if let Some(key) = map_get_resolved(&assets.map, "default", prefer_https) {
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

/// Normalize an image reference for Discord Assets.
///
/// - `https://…` → keep
/// - `http://…` → upgrade to `https://` when `prefer_https` (same host/path)
/// - `file://…` / other local → only if `serve_local` (still usually useless for Discord)
/// - portal key / bare string → `None` here (caller uses as key), except we return None
///   so callers can distinguish URL vs key via `is_https_url` / separate paths.
///
/// Returns `Some` only for network URLs Discord can fetch (or local when opted in).
fn normalize_image_ref(raw: &str, prefer_https: bool, serve_local: bool) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if s.starts_with("https://") {
        return Some(s.to_string());
    }
    if let Some(rest) = s.strip_prefix("http://") {
        if prefer_https {
            // Safe upgrade: same URL with https scheme (common for album CDNs).
            return Some(format!("https://{rest}"));
        }
        return Some(s.to_string());
    }
    if s.starts_with("file://") || s.starts_with('/') {
        if serve_local {
            // Documented limitation: Discord CDN rarely can fetch 127.0.0.1.
            // We do not start a loopback server unless/until verified; skip.
            debug!(
                path = s,
                "serve_local_art enabled but loopback hosting is not active; skipping local art"
            );
        }
        return None;
    }
    None
}

fn is_https_url(s: &str) -> bool {
    s.trim().starts_with("https://")
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

fn map_get_resolved(
    map: &std::collections::HashMap<String, String>,
    logical: &str,
    prefer_https: bool,
) -> Option<String> {
    let raw = map_get(map, logical)?;
    if let Some(url) = normalize_image_ref(&raw, prefer_https, false) {
        return Some(url);
    }
    // Portal asset key
    if prefer_https && is_https_url(&raw) {
        return Some(raw);
    }
    Some(raw)
}

fn resolve_app_asset(
    map: &std::collections::HashMap<String, String>,
    class: &str,
    prefer_https: bool,
) -> Option<String> {
    let normalized = normalize_class_key(class);
    let lower = class.to_lowercase();
    // Direct class / normalized lookup
    for candidate in [class, normalized.as_str(), lower.as_str()] {
        if let Some(v) = map_get_resolved(map, candidate, prefer_https) {
            return Some(v);
        }
    }
    // Aliases: VRChat, browsers, steam, …
    for alias in class_aliases(class) {
        if let Some(v) = map_get_resolved(map, alias, prefer_https) {
            return Some(v);
        }
    }
    None
}

fn class_aliases(class: &str) -> Vec<&'static str> {
    let c = class.to_lowercase();
    let mut out = Vec::new();
    if c.contains("vrchat") {
        out.push("vrchat");
    }
    if c.contains("firefox") {
        out.push("firefox");
    }
    if c.contains("chrom") || c == "google-chrome" || c == "brave-browser" || c == "helium" {
        if c.contains("brave") {
            out.push("brave-browser");
        } else if c == "helium" {
            out.push("helium");
            out.push("chrome");
        } else {
            out.push("chrome");
        }
    }
    if c.contains("steam") {
        out.push("steam");
    }
    if c.contains("spotif") {
        out.push("spotify");
    }
    if c == "code" || c.contains("code-url") || c.contains("codium") {
        out.push("code");
    }
    if c.contains("discord") || c.contains("vesktop") || c.contains("equibop") {
        if c.contains("equibop") {
            out.push("equibop");
        } else if c.contains("vesktop") {
            out.push("vesktop");
        } else {
            out.push("discord");
        }
    }
    out
}

fn normalize_class_key(class: &str) -> String {
    class
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn map_get_for_player(
    map: &std::collections::HashMap<String, String>,
    player: &str,
    prefer_https: bool,
) -> Option<String> {
    let normalized = normalize_player_key(player);
    if let Some(k) = map_get_resolved(map, &normalized, prefer_https) {
        return Some(k);
    }
    for alias in ["kopuz", "equibop", "spotify", "vlc"] {
        if player.to_lowercase().contains(alias) {
            if let Some(k) = map_get_resolved(map, alias, prefer_https) {
                return Some(k);
            }
        }
    }
    map_get_resolved(map, player, prefer_https)
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

fn truncate_tooltip(s: &str) -> String {
    if s.chars().count() > 128 {
        s.chars().take(125).collect::<String>() + "…"
    } else {
        s.to_string()
    }
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
    fn large_keeps_portal_key() {
        let cfg = cfg_with_assets();
        assert_eq!(resolve_large_asset(&cfg), "nixos");
    }

    #[test]
    fn large_prefers_https_url() {
        let mut cfg = cfg_with_assets();
        cfg.discord.assets.large_image = "https://cdn.simpleicons.org/nixos/5277C3".into();
        assert!(resolve_large_asset(&cfg).starts_with("https://"));
    }

    #[test]
    fn small_prefers_https_art_url_when_music_ours() {
        let cfg = cfg_with_assets();
        let state = State {
            art_url: Some("https://i.scdn.co/image/abc".into()),
            title: Some("Song".into()),
            player: Some("vlc".into()),
            ..State::default()
        };
        let (key, _) = resolve_small_asset(&cfg, &state, false, false);
        assert_eq!(key, "https://i.scdn.co/image/abc");
    }

    #[test]
    fn small_upgrades_http_art_url() {
        let cfg = cfg_with_assets();
        let state = State {
            art_url: Some("http://example.com/cover.png".into()),
            title: Some("Song".into()),
            ..State::default()
        };
        let (key, _) = resolve_small_asset(&cfg, &state, false, false);
        assert_eq!(key, "https://example.com/cover.png");
    }

    #[test]
    fn small_skips_file_art_by_default() {
        let cfg = cfg_with_assets();
        let state = State {
            art_url: Some("file:///tmp/cover.png".into()),
            title: Some("Song".into()),
            focused_app: Some("firefox".into()),
            ..State::default()
        };
        let (key, _) = resolve_small_asset(&cfg, &state, false, false);
        assert!(key.starts_with("https://"), "got {key}");
    }

    #[test]
    fn small_uses_focused_app_in_coexist_defer() {
        let cfg = cfg_with_assets();
        let state = State {
            art_url: Some("https://i.scdn.co/image/abc".into()),
            title: Some("Song".into()),
            player: Some("Kopuz".into()),
            focused_app: Some("firefox".into()),
            focused_title: Some("MDN".into()),
            ..State::default()
        };
        let (key, text) = resolve_small_asset(&cfg, &state, true, true);
        assert!(key.starts_with("https://"), "got {key}");
        assert!(key.contains("firefox") || key.contains("simpleicons"));
        assert_eq!(text, "MDN");
    }

    #[test]
    fn small_prefers_vrchat_when_running_no_focus_map() {
        let mut cfg = cfg_with_assets();
        // Empty focus path: no focused_app
        cfg.discord.assets.map.retain(|k, _| {
            !matches!(
                k.as_str(),
                "firefox"
                    | "chrome"
                    | "steam"
                    | "helium"
                    | "code"
                    | "spotify"
                    | "discord"
                    | "vesktop"
                    | "brave-browser"
                    | "chromium"
                    | "google-chrome"
                    | "firefox-esr"
                    | "code-url-handler"
            )
        });
        let state = State {
            vrchat_running: true,
            player: Some("Kopuz".into()),
            ..State::default()
        };
        let (key, text) = resolve_small_asset(&cfg, &state, true, false);
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
        let (key, text) = resolve_small_asset(&cfg, &state, true, false);
        assert_eq!(key, "kopuz");
        assert_eq!(text, "Kopuz");
    }

    #[test]
    fn small_falls_back_to_default_map() {
        let cfg = cfg_with_assets();
        let state = State::default();
        let (key, text) = resolve_small_asset(&cfg, &state, false, false);
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
            focused_app: Some("firefox".into()),
            ..State::default()
        };
        let (key, text) = resolve_small_asset(&cfg, &state, false, false);
        assert_eq!(key, "equibop");
        assert_eq!(text, "Equibop");
    }

    #[test]
    fn small_override_https() {
        let mut cfg = cfg_with_assets();
        cfg.discord.assets.small_image = Some("https://cdn.simpleicons.org/nixos/5277C3".into());
        let state = State::default();
        let (key, _) = resolve_small_asset(&cfg, &state, false, false);
        assert!(key.starts_with("https://"));
    }

    #[test]
    fn small_omitted_when_default_unset() {
        let mut cfg = cfg_with_assets();
        cfg.discord.assets.map = HashMap::new();
        let state = State::default();
        let (key, text) = resolve_small_asset(&cfg, &state, false, false);
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

    #[test]
    fn normalize_http_upgrade() {
        assert_eq!(
            normalize_image_ref("http://a.com/x.png", true, false).as_deref(),
            Some("https://a.com/x.png")
        );
        assert!(normalize_image_ref("file:///tmp/x.png", true, false).is_none());
        assert!(normalize_image_ref("nixos", true, false).is_none());
    }
}
