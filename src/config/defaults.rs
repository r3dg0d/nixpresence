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
# {custom} {hostname} {uptime} …
# Optional sections: {?music: …} only expands when music is non-empty.

[[pages]]
name = "status"
enabled = true
template = "{custom}{?music: ┆ 🎵 {music}}"
priority = 10

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

[[pages]]
name = "music"
enabled = true
template = "🎵 {music}"
priority = 20

[profiles.default]
pages = ["status", "system", "gpu", "music"]

[profiles.vrchat]
pages = ["status", "music", "system"]
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
# defer    = disable Discord output while preferred player is active
mode = "coexist"
prefer_players = ["Kopuz", "kopuz"]

# Rich Presence art assets. Keys must be uploaded in the Discord Developer Portal
# (Application → Rich Presence → Art Assets), or be HTTPS image URLs.
# See assets/discord/README.md for the NixOS snowflake PNG to upload as `nixos`.
[discord.assets]
large_image = "nixos"
large_text = "NixOS"
# small_image = ""          # optional fixed override (skips VRChat / player resolution)
# small_text = ""
[discord.assets.map]
vrchat = "vrchat"
kopuz = "kopuz"
equibop = "equibop"
default = "nixpresence"

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
custom = true

[custom]
message = ""
rotate = ["NixOS enjoyer", "compiled with love", "reproducible vibes"]
rotate_secs = 30.0

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
