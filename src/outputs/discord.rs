//! Discord Rich Presence via discord-rich-presence.
//! Never uses Kopuz's application id. Music mode defaults to coexist.
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

        if details == self.last_details && state_s == self.last_state {
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
        match client.set_activity(act) {
            Ok(()) => {
                self.last_details = details;
                self.last_state = state_s;
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
