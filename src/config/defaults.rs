//! Default config.toml text shipped to users.
pub fn default_config_toml() -> &'static str {
    r#"# nixpresence configuration
# XDG: ~/.config/nixpresence/config.toml
# Docs: see docs/ and `nixpresence config validate`

[general]
tick_secs = 1.0
separator = " ┆ "
prefix = ""
suffix = ""
ellipsis = "…"
active_profile = "default"

[privacy]
# Safe defaults: never leak identity into chatbox / Discord.
hide_hostname = true
hide_username = true
hide_ip = true
hide_ssid = true
redact_custom_patterns = []

[rotation]
mode = "round_robin"   # round_robin | priority | smart | static
interval_secs = 10.0
pause_on_custom = true

# Pages are composed via templates. Tokens: {os} {kernel} {cpu} {cpu_short}
# {ram} {gpu} {gpu_name} {gpu_temp} {gpu_util} {music} {artist} {title}
# {custom} {hostname} {uptime} {time} {location} {weather} {temp} …
# Optional sections: {?music: …} only expands when music is non-empty.

[[pages]]
name = "status"
enabled = true
template = "{custom}{?music: ┆ 🎵 {music}}"
priority = 10

[[pages]]
name = "discord_add"
enabled = true
template = "Add me on discord: vincentonpc"
priority = 15

[[pages]]
name = "music"
enabled = true
template = "🎵 {music}"
priority = 20

[[pages]]
name = "local"
enabled = true
template = "{time} ┆ {location}{?weather: ┆ {weather}}"
priority = 25

[[pages]]
name = "system"
enabled = true
template = "{os} ┆ {kernel} ┆ {cpu_short} ┆ {ram}"
priority = 40

[[pages]]
name = "gpu"
enabled = true
template = "{gpu} ┆ {gpu_temp}°C ┆ {gpu_util}%"
priority = 50

[profiles.default]
pages = ["status", "discord_add", "music", "local", "system", "gpu"]

[profiles.vrchat]
pages = ["status", "discord_add", "music", "local", "system"]
[profiles.vrchat.rotation]
mode = "round_robin"
interval_secs = 8.0

[auto_profile]
enabled = true
on_vrchat = "vrchat"
on_idle = "default"

[osc]
enabled = true
host = "127.0.0.1"
port = 9000
notify = false          # false = no chat SFX on ticker updates
force = false           # true = send even when VRChat not detected
dedup_secs = 12.0
only_when_vrchat = true

[discord]
# Discord Rich Presence. Create YOUR OWN application at
# https://discord.com/developers/applications
# NEVER reuse Kopuz's application id (1470087339639443658).
enabled = false
application_id = ""     # or env NIXPRESENCE_DISCORD_APP_ID
details_template = "{custom}"
state_template = "{os} · {gpu_name}"

[discord.music]
# coexist  = do not publish music when Kopuz (or prefer_players) owns Discord RPC
# takeover = nixpresence publishes music presence
# defer    = clear our Discord activity while preferred player is active
mode = "coexist"
prefer_players = ["Kopuz", "kopuz"]

# Rich Presence art: HTTPS URLs preferred; portal asset keys also work.
# See assets/discord/README.md (NixOS snowflake PNG → upload as key `nixos`).
# Localhost / file:// art is skipped by default (Discord CDN cannot fetch loopback).
[discord.assets]
large_image = "nixos"       # or https://… URL
large_text = "NixOS"
prefer_https = true
serve_local_art = false     # keep false until Discord is verified to accept 127.0.0.1
# small_image = ""          # optional fixed override (HTTPS or portal key)
# small_text = ""
[discord.assets.map]
# Built-in defaults are merged on load; override any key with HTTPS (preferred)
# or a portal asset key (set prefer_https = false if you rely on uploaded keys).
firefox = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/firefox.png"
chrome = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/google-chrome.png"
helium = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/google-chrome.png"
brave-browser = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/brave.png"
steam = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/steam.png"
discord = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/discord.png"
equibop = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/discord.png"
vesktop = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/discord.png"
code = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/vscode.png"
cursor = "https://cdn.simpleicons.org/cursor/ffffff"
ghostty = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/ghostty.png"
kitty = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/terminal.png"
thunar = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/files.png"
mpv = "https://cdn.simpleicons.org/mpv/ffffff"
obs = "https://cdn.simpleicons.org/obsstudio/302E31"
vrchat = "https://cdn.simpleicons.org/vrchat/ffffff"
kopuz = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/spotify.png"
default = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/nixos.png"

[music]
enabled = true
prefer_kopuz = true
mpris = true
# kopuz_socket = ""   # optional; default $XDG_RUNTIME_DIR/kopuz/kopuzd.sock
template = "{artist} - {title}"

[modules]
system = true
hardware = true
nixos = true
kernel = true
network = false
mpris = true
kopuz = true
vrchat = true
focused_app = true
custom = true
location = true
time = true
weather = true

[custom]
message = ""
rotate = ["NixOS enjoyer", "compiled with love", "reproducible vibes"]
rotate_secs = 30.0

# User-requested location (not auto-detected). Off by default for privacy.
[location]
enabled = false
text = ""
latitude = 0.0
longitude = 0.0

[time]
enabled = true
timezone = "America/Los_Angeles"
format = "%-I:%M %p %Z"

# Open-Meteo forecast (uses [location] lat/lon). Off until coords set.
[weather]
enabled = false
provider = "open_meteo"
cache_secs = 600.0
units = "fahrenheit"

[hardware]
nvml = true
gpu_index = 0

[ipc]
enabled = true
# socket = ""  # default $XDG_RUNTIME_DIR/nixpresence.sock

[logging]
level = "info"
"#
}
