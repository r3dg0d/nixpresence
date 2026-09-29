# MagicChatBox analysis (concepts only)

**Status:** research notes for an independent Linux implementation (`nixpresence`).  
**Clone:** `/workspace/nixpresence-research/vrcosc-magicchatbox`  
**Upstream:** https://github.com/BoiHanny/vrcosc-magicchatbox  
**License posture:** MagicChatBox is **not** OSI-open. We study **concepts** and **file locations**. We do **not** copy source, assets, OAuth client IDs, protected modules, or UI text.

---

## 1. Stack

MagicChatBox is a **Windows-only .NET / WPF** desktop app.

| Fact | Location / evidence |
| --- | --- |
| Target | `vrcosc-magicchatbox/MagicChatbox.csproj` — `net10.0-windows10.0.26100.0`, `UseWPF`, `UseWindowsForms`, `RuntimeIdentifier=win-x64`, `OutputType=WinExe` |
| Version observed | same csproj, `0.9.226` |
| UI | WPF pages under `vrcosc-magicchatbox/UI/` plus tray (`UI/Tray/`) |
| DI / modules | `Services/ServiceRegistration.cs`, `Services/ModuleHost.cs`, `Services/IModule.cs` |
| Settings | JSON, versioned, via `Core/Configuration/JsonSettingsProvider.cs` |
| OSC send | `Services/OscSenderService.cs` (CoreOSC UDP) |
| Line assembly | `Core/Osc/OscOutputBuilder.cs` |
| Tests | `MagicChatbox.Tests/` |
| Separate API project | `MagicChatboxAPI/` (ban / allowed-service surface — treat as protected) |

There is no Linux, macOS, or headless target. Hardware, media, window titles, DPAPI, SteamVR, Soundpad, and Voicemod all sit on Windows APIs.

---

## 2. License implications (must not fork)

`License.md` is a custom **source-available Software License Agreement** (effective 2025-03-22), **not** MIT/Apache/GPL. It looks MIT-like on first glance and then forbids the things we would actually want.

Hard constraints:

1. **Protected Components** must stay intact in any fork or derivative:
   - Monitoring and Ban API (VRChat moderation / remote kill).
   - Pulsoid heart-rate module (exclusive; no other HR service in a derivative).
2. A derivative must keep the SLA, TOS (`Security.md`), Pulsoid TOS links, and the prescribed attribution.
3. Circumventing, replacing, or duplicating Protected Components is a material breach.
4. The grant is personal and non-transferable; breach terminates rights immediately.

`nixpresence` will **not** be a fork, port, or derivative of MagicChatBox source. We implement a new Rust program under **MIT** (or Apache-2.0) that:

- talks to **documented public protocols** (VRChat OSC, MPRIS, Kopuz's local gRPC, Discord IPC);
- invents its own types, composer, config schema, TUI, and module list;
- never copies MagicChatBox `.cs` files, XAML, icons, OAuth client IDs, or Pulsoid / ban wiring.

Reading the clone for *ideas* is fine. Pasting or mechanically translating source is not.

---

## 3. Useful concepts (cite path, do not paste)

### 3.1 Modules as independent producers

Concept: each integration is a long-lived module with enable/start/stop/save, plus a thin **OSC provider** that turns current state into one text segment.

- Lifecycle: `Services/IModule.cs`, `Services/ModuleHost.cs`, `Services/ModuleBootstrapper.cs`
- Provider contract: `Core/Osc/IOscProvider.cs`
  - identity (`SortKey`, `UiKey`)
  - drop weight (`Priority`)
  - VR vs desktop gate (`IsEnabledForCurrentMode`)
  - `TryBuild(context) -> optional segment`
- Segment: `Core/Osc/OscSegment.cs` — full text plus optional compact form
- Context: `Core/Osc/OscBuildContext.cs` — already-accepted segments, separator, prefix/suffix, VR flag, remaining-character helper
- Result: `Core/Osc/OscBuildResult.cs` — final line, included keys, trimmed keys, per-segment lengths, overflow flag
- Human names: `Core/Osc/OscProviderNames.cs`

Provider files (one per integration): `Core/Osc/Providers/*.cs`.

### 3.2 OSC builder and the 144-character priority drop

Concept location: `Core/Osc/OscOutputBuilder.cs`  
Limit constant: `Core/Constants.cs` (`OscMaxMessageLength = 144`, matching VRChat).

Observed algorithm (described, not copied):

1. Resolve separator, prefix, suffix. Expand literal `\n` to newlines.
2. Walk the **user sort order**, then a safety-net pass for unlisted providers.
3. Skip a provider if it is disabled for the current VR/desktop mode or currently faulted.
4. Ask the provider for a segment. Empty/null is skipped. Faults increment a tracker instead of crashing the scan.
5. While the assembled line (prefix + join(separator) + suffix) is over 144 **and more than one segment remains**, drop the collected segment with the **highest numeric Priority**.
6. If a single survivor still overflows, clip it to the leftover budget (ellipsis, surrogate-safe) and hard-clamp the final string.

`Priority` here is a **drop weight**: larger number is discarded first. Observed weights (from each provider file):

| Drop weight | Provider `UiKey` / `SortKey` |
| --- | --- |
| 90 | Time |
| 85 | Weather |
| 80 | Voicemod, Network stats |
| 75 | Soundpad |
| 20–70 | Lyrics 22, MediaLink 20, Spotify 25, Window 30, VRChat radar 35, Heart 40, Discord 45, Twitch 50, TikTok 52, Tracker battery 60, VR perf 65, Component stats 70 |
| 10 | Personal status (kept longest) |

README states the product intent in English: drop queue / volume / device before the song title. The builder encodes that as numeric weights plus optional compact text, not as mid-word truncation of the whole line.

Also in the same file:

- default separator is a short unicode bar with spaces;
- optional "separate with enters" mode uses `\n` (VRChat allows up to 9 lines);
- prefix/suffix are user-configurable.

### 3.3 VR vs desktop profiles

Concept: a master integration toggle **and** two mode switches.

- Settings bag: `Classes/Modules/IntegrationSettings.cs`
- Mode table: `Classes/Modules/IntegrationModeVisibility.cs` (`*_VR` / `*_DESKTOP` pairs)
- Runtime flag: `IAppState.IsVRRunning` (passed into `IsEnabledForCurrentMode`)

Effect: the same process can show GPU temps at the desk and hide them in headset, without the user flipping toggles when they put the HMD on.

### 3.4 Discord

Two different jobs live under one brand name:

1. **Chatbox segment** — voice channel / talking names: `Core/Osc/Providers/DiscordOscProvider.cs`
2. **Discord Rich Presence** — push VRChat world / player count / join flavor into Discord: `Services/DiscordRichPresenceService.cs`

IPC constants live in `Core/Constants.cs` (`discord-ipc-` pipes 0–9, reconnect backoff). That is Discord's documented local RPC, not MagicChatBox-specific protocol. **Do not reuse their Discord application ID.** Register our own.

### 3.5 Music

Two sources, same chatbox slot family:

- **MediaLink** — OS "now playing" (Windows media session): `Classes/Modules/MediaLinkModule.cs`, `Classes/Modules/Media/`, `Core/Osc/Providers/MediaLinkOscProvider.cs`
- **Spotify Web API** — extra fields (queue, device, volume, explicit, liked): `Classes/Modules/SpotifyModule.cs`, `Classes/Modules/Spotify/`, `Core/Osc/Providers/SpotifyOscProvider.cs`

MediaLink extras (conceptual): title cleaner, artist shortener, timeline policy, "show only on change" transient window (`Core/Osc/TransientWindow.cs`), lyrics suppression of the title, uppercase option.

Lyrics are a follower segment: `Core/Osc/Providers/LyricsOscProvider.cs`, `Services/Lyrics/`, default sort marks `"Lyrics"` as a follower of media (`ViewModels/State/IntegrationDisplayState.cs`).

**Linux mapping:** do not port SMTC or copy Spotify OAuth. Use Kopuz gRPC first, MPRIS second (see `kopuz-integration.md`).

### 3.6 Component stats

- Module: `Classes/Modules/ComponentStatsModule.cs`
- Provider: `Core/Osc/Providers/ComponentStatsOscProvider.cs`
- Hardware IO: `Services/HardwareMonitorService.cs`, `Services/Hardware/`

Display order concept: CPU, GPU, VRAM, RAM. The provider asks the module to **write within the remaining budget** rather than emitting a fixed string and hoping. Sensors on Windows come from LibreHardwareMonitor, `nvidia-smi`, and Performance Counters — none of that is usable on Linux as-is.

### 3.7 Rotation / order

User order is a string list of sort keys, persisted on `IntegrationSettings` and snapshotted on `IntegrationDisplayState`. Default order (keys only): Status, Window, Twitch, TikTokLive, Discord, Spotify, VrcRadar, HeartRate, Component, VrPerformance, TrackerBattery, Network, Weather, Time, Soundpad, Voicemod, MediaLink, Lyrics.

Reorder UI: `UI/Dialogs/ReorderIntegrations.xaml.cs`.  
Scan loop that rebuilds and sends: `Services/ScanLoopService.cs`.

### 3.8 Separators, prefix, suffix

Configured on app settings (`Classes/Modules/AppSettings.cs` conceptually; consumed in `OscOutputBuilder`). Preview of those decorations without a live scan: `Core/Osc/OscLinePreview.cs`.

### 3.9 Preview

- Static decoration preview: `Core/Osc/OscLinePreview.cs`
- Fill classification: `Core/Osc/OscPreviewFill.cs` — roomy / tight (≥ 85%) / full
- Live result presentation: `Core/Osc/OscBuildResultPresenter.cs`
- WPF widget: `UI/Controls/SegmentPreview.xaml.cs`

For nixpresence the equivalent is a ratatui pane, not a WPF control.

### 3.10 Diagnostics and resilience

- Fault isolation: `Core/Osc/ModuleFaultTracker.cs` — consecutive-failure trip, cooldown, single probe
- Constants for that policy: `Core/Constants.cs` (`ModuleMaxConsecutiveFailures = 3`, 60s cooldown)
- UI / perf probes: `Core/Diagnostics/*` (WPF-specific; do not port)
- Sender keep-alive: `Services/OscSenderService.cs` — skip identical payloads except a periodic resend so VRChat does not expire the chatbox
- Multi-output OSC: `Classes/Modules/OscSettings.cs` — primary `127.0.0.1:9000`, optional second/third endpoints

---

## 4. What we implement independently

Independent Rust types, no shared code:

| Capability | nixpresence approach |
| --- | --- |
| Module + provider split | Our `Source` trait + composer, not `IOscProvider` |
| 144-char budget | Our grapheme-aware packer (`unicode-segmentation`) |
| Priority drop | Our weights + user order in TOML |
| VR / desktop profiles | Detect VR runtime or SteamVR/OpenXR; two enable maps |
| Music | Kopuz gRPC, then MPRIS (`zbus`) |
| Hardware | `nvml-wrapper`, hwmon, `/proc` |
| Discord RPC | Our application ID, `discord-sdk` or `discord-rich-presence` |
| OSC send | `rosc` UDP to 9000, optional listen on 9001 |
| Preview / diagnostics | `ratatui` + `tracing` |
| Config | XDG TOML, serde |

---

## 5. Redesigns for Linux

| Windows concept | Linux redesign |
| --- | --- |
| WPF + tray | CLI (`clap`) + TUI (`ratatui`); optional later tray |
| WinRT / SMTC media | MPRIS via `zbus`; Kopuz socket as first-class source |
| LibreHardwareMonitor + WMI | NVML, hwmon, `/proc/stat`, `/proc/meminfo` |
| `discord-ipc-N` named pipes | Discord IPC unix sockets under `$XDG_RUNTIME_DIR` |
| DPAPI token store | file mode 0600 under XDG; optional secret-service later |
| VRChat log under `%LOCALAPPDATA%` | Proton prefix: `.../compatdata/<appid>/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/` |
| OpenVR / SteamVR batteries | optional; SteamVR on Linux is a different integration |
| `localhost` hostnames | always `127.0.0.1` (see `vrchat-osc-notes.md`) |
| `%APPDATA%` settings | `$XDG_CONFIG_HOME/nixpresence/` |

---

## 6. Intentional omissions

Do **not** implement, port, or "clean-room" these:

| Item | Why |
| --- | --- |
| Pulsoid module / OAuth / client id | SLA Protected Component (`License.md` §5; constants in `Core/Constants.cs`) |
| Ban / Monitoring / Moderation API | SLA Protected Component (`Services/IBanEnforcementService.cs`, `Services/BanEnforcementService.cs`, `MagicChatboxAPI/`) |
| Soundpad named-pipe client | Windows-only (`Services/SoundpadPipeClient.cs`) |
| Voicemod Control API + embedded key | Windows + third-party key (`Services/Voicemod/`, csproj `VoicemodClientKey`) |
| Spotify OAuth client / redirect ports / scope string | Do not copy `Core/Constants.cs` or `Classes/Modules/SpotifyOAuthHandler.cs`. If we ever talk to Spotify, register a new app. |
| Window Activity (Win32 titles) | Privacy-sensitive and Win32-specific. Optional later via `wlr-foreign-toplevel` / X11; default off. |
| IntelliChat / OpenAI | Out of v1 scope; their org/key UI is not ours |
| TTS / speech-to-text / Whisper | Windows audio stack; not v1 |
| TikTok Live, Twitch Helix, OpenWeather keys | Product bloat; add later with our own credentials if wanted |
| Auto-update / GitHub release checker | Different distribution (Nix) |
| WPF diagnostics / visual-tree census | Irrelevant |
| Heart-rate avatar float parameters | Only meaningful with Pulsoid in the original app |
| Any "blank suffix" / undocumented chatbox padding | Do not reproduce undocumented payload tricks |

---

## 7. Technical differences (nixpresence vs MagicChatBox)

| | MagicChatBox | nixpresence |
| --- | --- | --- |
| Language | C# / .NET 10 | Rust |
| UI | WPF | ratatui + clap |
| OS | Windows 10/11 x64 | Linux (Nix-friendly) |
| License | proprietary SLA | MIT |
| Music | SMTC + Spotify OAuth | Kopuz gRPC + MPRIS |
| GPU | LHM / nvidia-smi.exe | NVML + hwmon |
| Chatbox | CoreOSC | rosc |
| Config | JSON next to the exe / AppData | XDG TOML |
| Discord | Lachee DiscordRPC + their app id | our app id |
| Process model | one WinExe | one async tokio binary, optional TUI |
| VRChat host | native Win64 | Proton / Wine; same UDP loopback |

---

## 8. Rule for later implementers

When in doubt: **re-read this file, then write new Rust.**  
If a function would be easier to produce by opening a `.cs` file and translating it, stop. Describe the behavior in a sentence and write it from the sentence.
