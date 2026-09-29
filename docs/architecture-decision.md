# Architecture decision: nixpresence

**Decision:** new Rust workspace, MIT, Linux-first, independently implemented.  
**Not a port** of MagicChatBox (proprietary SLA). See `magicchatbox-analysis.md`.  
**Music:** Kopuz gRPC, then MPRIS. See `kopuz-integration.md`.  
**Wire:** VRChat `/chatbox/input` on UDP 9000. See `vrchat-osc-notes.md`.

---

## 1. Goals (v1)

- Compose a VRChat chatbox line from music, hardware, clock, and an optional user status.
- Send it over OSC from a native Linux process to Proton VRChat on loopback.
- Optional Discord Rich Presence (our application id).
- Optional ratatui preview / diagnostics.
- Nix-friendly: one binary, XDG paths, no homedir litter, no phone-home.

Non-goals for v1: Pulsoid, ban API, Spotify OAuth, Twitch/TikTok, TTS, WPF clone, Windows build.

---

## 2. License

**MIT** for the workspace and every crate (Apache-2.0 is the fallback if a future dependency forces dual-license; prefer MIT-only until then).

Third-party:

| Dependency | Typical license | Role |
| --- | --- | --- |
| clap, tokio, serde, toml, tracing | MIT / Apache-2.0 | CLI, runtime, config, logs |
| rosc | MIT/Apache | OSC encode |
| zbus | MIT | MPRIS |
| nvml-wrapper | MIT/Apache | NVIDIA sensors |
| unicode-segmentation | MIT/Apache | 144-char clip |
| ratatui | MIT | TUI |
| discord-sdk **or** discord-rich-presence | MIT/Apache / Apache-2.0 | Discord IPC |
| tonic + prost (optional) | MIT | Kopuz client |

Do not vendor MagicChatBox source. Do not vendor Kopuz; generate from a **dated proto snapshot** or speak MPRIS only.

---

## 3. XDG paths

Follow the Base Directory spec. Override with `NIXPRESENCE_CONFIG`, `NIXPRESENCE_STATE`, etc. if set.

| Kind | Default |
| --- | --- |
| Config | `$XDG_CONFIG_HOME/nixpresence/config.toml` → `~/.config/nixpresence/config.toml` |
| State | `$XDG_STATE_HOME/nixpresence/` (last line, TUI layout) |
| Cache | `$XDG_CACHE_HOME/nixpresence/` (proto cache, cover fetch if any) |
| Runtime | `$XDG_RUNTIME_DIR/nixpresence/` (lock file so two composers do not fight) |
| Data | `$XDG_DATA_HOME/nixpresence/` (optional status snippets) |

Config file mode `0600` when we write tokens (none in v1). Never store secrets next to the binary.

Kopuz socket we **read**, not own: `$XDG_RUNTIME_DIR/kopuz/kopuzd.sock`.

---

## 4. Crate layout

Cargo workspace at the repo root. One user-facing binary.

```text
nixpresence/
├── Cargo.toml                 # workspace, license = "MIT"
├── LICENSE
├── README.md
├── flake.nix                  # optional; package the binary
└── crates/
    ├── nixpresence/           # bin: clap, tokio::main, tracing-subscriber
    ├── nixpresence-core/      # types, errors, Profile (Vr/Desktop), XDG helpers
    ├── nixpresence-config/    # serde + toml, defaults, validate
    ├── nixpresence-compose/   # segments, order, drop weights, 144 pack
    ├── nixpresence-osc/       # rosc UDP sender
    ├── nixpresence-music/     # Kopuz gRPC + MPRIS (zbus)
    ├── nixpresence-hw/        # nvml-wrapper + /proc + hwmon
    ├── nixpresence-discord/   # discord-sdk or discord-rich-presence
    └── nixpresence-tui/       # ratatui preview / live diagnostics
```

Why split:

- `compose` has no IO — easy to unit-test the 144-char packer.
- `osc` / `music` / `hw` / `discord` fail independently; the composer skips a dead source.
- `tui` is optional (`--tui` / default when stdout is a tty). Headless `nixpresence run` is the Steam-launch companion.

Binary crate `crates/nixpresence` is allowed to depend on every other crate. Sibling crates do **not** depend on the binary. `music` must not depend on `discord`. `discord` may read a `NowPlaying` DTO from `core`.

---

## 5. Crate roles and dependencies

### `nixpresence` (bin)

- `clap` derive: `run`, `print` (compose once to stdout), `doctor` (ports, socket, NVML, Discord IPC).
- `tokio` full: one runtime, interval tick (compose 2–5 Hz; send on change).
- `tracing` + `tracing-subscriber` (env filter `NIXPRESENCE_LOG`).
- Wires sources → composer → osc (+ discord + tui).

### `nixpresence-core`

Owned types only:

```text
Profile          = Desktop | Vr
Segment          { id, text, drop_weight }
ComposeRequest   { profile, order, prefix, suffix, separator, limit=144 }
NowPlaying       { title, artist, album, position, duration, playing, source }
HwSample         { cpu_pct, gpu_pct, gpu_c, vram_pct, ram_pct, ... }
```

XDG path helpers live here so config and runtime share them.

### `nixpresence-config`

`serde` + `toml`. Example surface:

```toml
[osc]
host = "127.0.0.1"
port = 9000
notify = false
keepalive_secs = 12

[compose]
separator = " ┆ "
prefix = ""
suffix = ""
order = ["status", "music", "hw", "time"]

[profile.desktop]
music = true
hw = true
time = true
status = true

[profile.vr]
music = true
hw = false
time = true
status = true

[music]
backend = "auto"          # auto | kopuz | mpris
mpris_identity = "kopuz"

[hw]
nvidia = true

[discord]
enabled = false
mode = "auto"             # off | vrchat | listening | auto
# application_id set at build or in this table (ours, never Kopuz's, never MagicChatBox's)

[tui]
enabled = true
```

### `nixpresence-compose`

Pure function:

1. Filter segments by profile enable map.
2. Sort by user `order` (unknown ids append).
3. Join with separator; while `grapheme_len(prefix+body+suffix) > 144` and `len(segments) > 1`, drop the highest `drop_weight`.
4. If one remains, ellipsis-clip on a grapheme boundary (`unicode-segmentation`).
5. Return `{ line, included, dropped, graphemes }`.

Suggested default weights (ours, not copied as code): status 10, music 20, time 80, hw 70. User can override.

### `nixpresence-osc`

- `rosc` encode `/chatbox/input` string + send-bool + notify-bool.
- IPv4 UDP only. `send_changed(line)` + keepalive.
- Optional `/chatbox/typing` helper.
- No OSCQuery in v1.

### `nixpresence-music`

```text
auto → try kopuz unix socket
     → else zbus MPRIS (prefer configured identity, else first Playing)
```

Kopuz: tonic client, `Subscribe` then `GetPlayerState`, interpolate position. Soft-fail on proto mismatch.  
MPRIS: `zbus` proxy to `org.mpris.MediaPlayer2.*`.

### `nixpresence-hw`

- NVIDIA: `nvml-wrapper` (temp, util, VRAM). Absent driver → skip GPU, do not crash.
- CPU / RAM: `/proc/stat`, `/proc/meminfo`.
- Optional later: hwmon junction temps for AMD.

Tick 2 s. Cache last good sample.

### `nixpresence-discord`

Pick **one** crate at implementation time:

| Crate | Why pick it |
| --- | --- |
| `discord-rich-presence` | small, proven in Kopuz; IPC set_activity |
| `discord-sdk` | richer, async-native, more moving parts |

Recommendation: **`discord-rich-presence`** for v1 (thin, matches how every Linux music app talks IPC). Revisit `discord-sdk` if we need events.

Register **our** Discord application. 30 s reconnect tick is a reasonable keepalive (Discord expires stale activities). Honor coexistence modes from `kopuz-integration.md`.

### `nixpresence-tui`

`ratatui` + crossterm:

- live line + grapheme count + fill (roomy / tight / full)
- which segments survived
- music / hw / osc / discord health
- log tail from a `tracing` layer

Headless mode skips this crate's enter/leave alternate screen.

---

## 6. Runtime picture

```text
tokio::select
  ├─ music events (gRPC stream or MPRIS PropertiesChanged)
  ├─ hw interval (2s)
  ├─ clock interval (30s is enough; 1s if seconds are shown)
  ├─ discord tick (30s)
  ├─ osc keepalive (12s)
  └─ tui input

on any source change:
  compose(profile) → if line != last || keepalive due → osc.send(line, send=true, notify=false)
```

Profile flip: SteamVR/OpenXR process present, or a config override `--profile vr|desktop|auto`.

Single-instance lock: `$XDG_RUNTIME_DIR/nixpresence/lock`. Second process exits with a clear error.

---

## 7. Testing

| Crate | Tests |
| --- | --- |
| compose | table-driven: overflow drops hw before music; grapheme clip; prefix+suffix budget; 9-line cap |
| osc | encode bytes of `/chatbox/input` (no live VRChat) |
| music | mock MPRIS; optional ignored `#[cfg]` Kopuz test if socket exists |
| config | parse defaults, reject empty host, clamp port |
| bin `doctor` | smoke: bind UDP 0, resolve XDG, list MPRIS names |

No test may open MagicChatBox sources.

---

## 8. Distribution

- `cargo build --release -p nixpresence`
- Nix flake later: wrap with `nvml` / Discord optional features  
  `--features hw-nvidia,discord,tui,music-kopuz`
- Default features: `tui`, `music-mpris`, `osc`. Kopuz + NVML + Discord are features so a laptop without NVIDIA still builds.

---

## 9. Decisions locked

1. Independent implementation, MIT, no MagicChatBox source.
2. Workspace crates as in §4.
3. Composer is grapheme-aware, drop-weight, user order.
4. Music: Kopuz gRPC then MPRIS.
5. OSC: IPv4 `127.0.0.1:9000`, notify false on ticks.
6. Discord optional, our app id, coexist with Kopuz.
7. XDG only; no extra directories in `$HOME`.
