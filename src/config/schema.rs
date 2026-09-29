use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const OSC_MAX_GRAPHEMES: usize = 144;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub privacy: PrivacyConfig,
    pub rotation: RotationConfig,
    pub pages: Vec<PageConfig>,
    pub profiles: HashMap<String, ProfileConfig>,
    pub auto_profile: AutoProfileConfig,
    pub osc: OscConfig,
    pub discord: DiscordConfig,
    pub music: MusicConfig,
    pub modules: ModulesConfig,
    pub custom: CustomConfig,
    pub hardware: HardwareConfig,
    pub ipc: IpcConfig,
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            privacy: PrivacyConfig::default(),
            rotation: RotationConfig::default(),
            pages: default_pages(),
            profiles: default_profiles(),
            auto_profile: AutoProfileConfig::default(),
            osc: OscConfig::default(),
            discord: DiscordConfig::default(),
            music: MusicConfig::default(),
            modules: ModulesConfig::default(),
            custom: CustomConfig::default(),
            hardware: HardwareConfig::default(),
            ipc: IpcConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

fn default_pages() -> Vec<PageConfig> {
    vec![
        PageConfig {
            name: "status".into(),
            enabled: true,
            template: "{custom}{?music: ┆ 🎵 {music}}".into(),
            priority: 10,
            interval_secs: None,
        },
        PageConfig {
            name: "system".into(),
            enabled: true,
            template: "{os} ┆ {kernel} ┆ {cpu_short} ┆ {ram}".into(),
            priority: 40,
            interval_secs: None,
        },
        PageConfig {
            name: "gpu".into(),
            enabled: true,
            template: "{gpu} ┆ {gpu_temp}°C ┆ {gpu_util}%".into(),
            priority: 50,
            interval_secs: None,
        },
        PageConfig {
            name: "music".into(),
            enabled: true,
            template: "🎵 {music}".into(),
            priority: 20,
            interval_secs: None,
        },
    ]
}

fn default_profiles() -> HashMap<String, ProfileConfig> {
    let mut m = HashMap::new();
    m.insert(
        "default".into(),
        ProfileConfig {
            pages: vec![
                "status".into(),
                "system".into(),
                "gpu".into(),
                "music".into(),
            ],
            rotation: None,
        },
    );
    m.insert(
        "vrchat".into(),
        ProfileConfig {
            pages: vec!["status".into(), "music".into(), "system".into()],
            rotation: Some(RotationConfig {
                mode: RotationMode::RoundRobin,
                interval_secs: 8.0,
                ..RotationConfig::default()
            }),
        },
    );
    m
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub tick_secs: f64,
    pub separator: String,
    pub prefix: String,
    pub suffix: String,
    pub ellipsis: String,
    pub active_profile: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            tick_secs: 1.0,
            separator: " ┆ ".into(),
            prefix: String::new(),
            suffix: String::new(),
            ellipsis: "…".into(),
            active_profile: "default".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    /// Never publish hostname / username / IP by default.
    pub hide_hostname: bool,
    pub hide_username: bool,
    pub hide_ip: bool,
    pub hide_ssid: bool,
    pub redact_custom_patterns: Vec<String>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            hide_hostname: true,
            hide_username: true,
            hide_ip: true,
            hide_ssid: true,
            redact_custom_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RotationMode {
    #[default]
    RoundRobin,
    Priority,
    Smart,
    Static,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RotationConfig {
    pub mode: RotationMode,
    pub interval_secs: f64,
    pub pause_on_custom: bool,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            mode: RotationMode::RoundRobin,
            interval_secs: 10.0,
            pause_on_custom: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageConfig {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub template: String,
    #[serde(default = "default_priority")]
    pub priority: u32,
    pub interval_secs: Option<f64>,
}

fn default_true() -> bool {
    true
}
fn default_priority() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub pages: Vec<String>,
    pub rotation: Option<RotationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoProfileConfig {
    pub enabled: bool,
    pub on_vrchat: String,
    pub on_idle: String,
}

impl Default for AutoProfileConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            on_vrchat: "vrchat".into(),
            on_idle: "default".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OscConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub notify: bool,
    pub force: bool,
    pub dedup_secs: f64,
    pub only_when_vrchat: bool,
}

impl Default for OscConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".into(),
            port: 9000,
            notify: false, // avoid SFX spam for tickers
            force: false,
            dedup_secs: 12.0,
            only_when_vrchat: true,
        }
    }
}

/// Discord Rich Presence art assets.
///
/// `large_image` / `small_image` may be **asset keys** uploaded in the Discord
/// Developer Portal (Application → Rich Presence → Art Assets), or **HTTPS**
/// image URLs (preferred when `prefer_https` is true). The
/// `discord-rich-presence` crate accepts both.
///
/// Localhost / `file://` art is generally **not** fetchable by Discord's CDN.
/// `serve_local_art` stays off until verified; see `assets/discord/README.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscordAssetsConfig {
    /// Large image: HTTPS URL (preferred) or portal asset key (default `nixos`).
    pub large_image: String,
    /// Tooltip for the large image.
    pub large_text: String,
    /// Optional fixed small-image override (skips dynamic resolution).
    pub small_image: Option<String>,
    /// Optional tooltip for the small image.
    pub small_text: Option<String>,
    /// Prefer HTTPS URLs over portal keys when a value looks like `https://`.
    pub prefer_https: bool,
    /// Serve local `file://` / icon bytes on `127.0.0.1` (default **false** —
    /// Discord typically cannot fetch loopback from their CDN path).
    pub serve_local_art: bool,
    /// Logical name / window class → Discord asset key **or** HTTPS URL.
    /// Keys: `vrchat`, `kopuz`, `firefox`, `chrome`, `steam`, `default`, …
    pub map: HashMap<String, String>,
}

fn default_asset_map() -> HashMap<String, String> {
    let mut m = HashMap::new();
    // Portal keys (upload in Developer Portal) — override with HTTPS in config if desired.
    m.insert("vrchat".into(), "vrchat".into());
    m.insert("kopuz".into(), "kopuz".into());
    m.insert("equibop".into(), "equibop".into());
    m.insert("default".into(), "nixpresence".into());
    // Common focused-app classes → public HTTPS icons (Simple Icons CDN).
    m.insert(
        "firefox".into(),
        "https://cdn.simpleicons.org/firefox/FF7139".into(),
    );
    m.insert(
        "firefox-esr".into(),
        "https://cdn.simpleicons.org/firefox/FF7139".into(),
    );
    m.insert(
        "chrome".into(),
        "https://cdn.simpleicons.org/googlechrome/4285F4".into(),
    );
    m.insert(
        "google-chrome".into(),
        "https://cdn.simpleicons.org/googlechrome/4285F4".into(),
    );
    m.insert(
        "chromium".into(),
        "https://cdn.simpleicons.org/googlechrome/4285F4".into(),
    );
    m.insert(
        "brave-browser".into(),
        "https://cdn.simpleicons.org/brave/FB542B".into(),
    );
    m.insert(
        "steam".into(),
        "https://cdn.simpleicons.org/steam/ffffff".into(),
    );
    m.insert(
        "spotify".into(),
        "https://cdn.simpleicons.org/spotify/1DB954".into(),
    );
    m.insert(
        "code".into(),
        "https://cdn.simpleicons.org/visualstudiocode/007ACC".into(),
    );
    m.insert(
        "code-url-handler".into(),
        "https://cdn.simpleicons.org/visualstudiocode/007ACC".into(),
    );
    m.insert(
        "discord".into(),
        "https://cdn.simpleicons.org/discord/5865F2".into(),
    );
    m.insert(
        "vesktop".into(),
        "https://cdn.simpleicons.org/discord/5865F2".into(),
    );
    m.insert(
        "helium".into(),
        "https://cdn.simpleicons.org/googlechrome/4285F4".into(),
    );
    m
}

impl Default for DiscordAssetsConfig {
    fn default() -> Self {
        Self {
            large_image: "nixos".into(),
            large_text: "NixOS".into(),
            small_image: None,
            small_text: None,
            prefer_https: true,
            serve_local_art: false,
            map: default_asset_map(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscordConfig {
    pub enabled: bool,
    /// Your Discord Application ID. Do NOT use Kopuz's ID (1470087339639443658).
    /// Create one at https://discord.com/developers/applications
    /// Or set NIXPRESENCE_DISCORD_APP_ID for tests.
    pub application_id: String,
    pub details_template: String,
    pub state_template: String,
    pub music: DiscordMusicConfig,
    pub assets: DiscordAssetsConfig,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false, // off until user sets application_id
            application_id: String::new(),
            details_template: "{custom}".into(),
            state_template: "{os} · {gpu_name}".into(),
            music: DiscordMusicConfig::default(),
            assets: DiscordAssetsConfig::default(),
        }
    }
}

/// How nixpresence music interacts with players that already own Discord RPC
/// (e.g. Kopuz with discord_presence = true).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DiscordMusicMode {
    /// Never publish music presence; leave it to Kopuz / other players.
    #[default]
    Coexist,
    /// Clear and take over music presence.
    Takeover,
    /// Disable Discord output entirely when a preferred music player is active.
    Defer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscordMusicConfig {
    pub mode: DiscordMusicMode,
    /// Player names (MPRIS identity) treated as owning Discord music RPC.
    pub prefer_players: Vec<String>,
}

impl Default for DiscordMusicConfig {
    fn default() -> Self {
        Self {
            mode: DiscordMusicMode::Coexist,
            prefer_players: vec!["Kopuz".into(), "kopuz".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MusicConfig {
    pub enabled: bool,
    pub prefer_kopuz: bool,
    pub mpris: bool,
    /// Optional gRPC UDS (best-effort; not required).
    pub kopuz_socket: Option<String>,
    pub template: String,
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefer_kopuz: true,
            mpris: true,
            kopuz_socket: None,
            template: "{artist} - {title}".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModulesConfig {
    pub system: bool,
    pub hardware: bool,
    pub nixos: bool,
    pub kernel: bool,
    pub network: bool,
    pub mpris: bool,
    pub kopuz: bool,
    pub vrchat: bool,
    pub focused_app: bool,
    pub custom: bool,
}

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            system: true,
            hardware: true,
            nixos: true,
            kernel: true,
            network: false, // privacy-safe default
            mpris: true,
            kopuz: true,
            vrchat: true,
            focused_app: true,
            custom: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomConfig {
    pub message: String,
    pub rotate: Vec<String>,
    pub rotate_secs: f64,
}

impl Default for CustomConfig {
    fn default() -> Self {
        Self {
            message: String::new(),
            rotate: vec![
                "NixOS enjoyer".into(),
                "compiled with love".into(),
                "reproducible vibes".into(),
            ],
            rotate_secs: 30.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HardwareConfig {
    pub nvml: bool,
    pub gpu_index: u32,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            nvml: true,
            gpu_index: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IpcConfig {
    pub enabled: bool,
    pub socket: Option<String>,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            socket: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
        }
    }
}
