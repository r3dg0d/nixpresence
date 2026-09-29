# Kopuz integration notes

**Status:** how `nixpresence` should read now-playing from Kopuz, and how the two programs coexist.  
**Clone:** `/workspace/nixpresence-research/kopuz`  
**Docs:** `docs/api.md`  
**Contract file:** `crates/proto/proto/kopuz.proto` (package `kopuz.v1`)  
**Kopuz license:** EUPL-1.2 (`LICENSE`). We are an out-of-process client of a local socket, not a fork.

---

## 0. Schema instability (read this first)

`docs/api.md` is explicit:

> This is a local IPC channel between the daemon and a frontend on the same machine. It is never exposed to a network, and it is **not a stable public API** — the schema changes with the app, in the same commit.

Consequences for nixpresence:

- Treat `kopuz.v1` as **best-effort**. Pin the proto we generate against, and fail soft when fields vanish.
- Ignore unknown `EventEnvelope` oneofs and unknown enum values (the proto comments require this).
- Prefer a **narrow** client: `GetStatus`, `GetPlayerState`, `Subscribe`. Do not take a dependency on library/scan/playlist RPCs.
- Always keep an **MPRIS fallback**. When Kopuz is upgraded, missing, or headless-without-socket, music still works.
- Do not advertise "Kopuz support" as a stable ABI in our own docs. Call it "Kopuz daemon, when present."

---

## 1. Process shapes

From `docs/api.md`:

| Shape | Binary | Who serves the API |
| --- | --- | --- |
| Headless | `kopuzd` | daemon process |
| Embedded | desktop `kopuz` app | same core, in-process, same socket/API |

SQLite is single-writer. `kopuzd` and the GUI must not open the same library. A frontend that wants live state while the GUI is open **attaches to the GUI's embedded server**, not a second daemon.

Flags: `--socket`, `--listen`, `--token-file`, `--db-path`. Exclusive lock file beside the DB.

---

## 2. Socket rendezvous

Linux path (the one we care about):

```text
$XDG_RUNTIME_DIR/kopuz/kopuzd.sock
```

Created `0600`. **No token, no discovery file, no port** on the unix socket. The kernel file mode is the ACL.

Other platforms (for awareness only):

- macOS: user cache dir, `~/Library/Caches/kopuz/kopuzd.sock`
- Windows: named pipe `\\.\pipe\kopuz-<user SID>`

Stale socket: connect refused → daemon unlinks and binds. Non-socket at that path is left alone. Second listener gets `AddrInUse`.

Optional TCP (`kopuzd --listen ip:port`) requires `authorization: Bearer <token>` from `$XDG_RUNTIME_DIR/kopuz/kopuzd.token`. **nixpresence should not use TCP.** Stay on the unix socket.

Reflection is enabled. Manual probe:

```sh
SOCK=$XDG_RUNTIME_DIR/kopuz/kopuzd.sock
grpcurl -unix -plaintext "$SOCK" list kopuz.v1.Kopuz
grpcurl -unix -plaintext "$SOCK" kopuz.v1.Kopuz/GetPlayerState
```

Rust client in-tree: `crates/client` (`client::GrpcApi`). We can generate our own tonic stubs from the proto **or** speak a tiny hand-rolled client. Generating from a vendored snapshot of `kopuz.proto` (with a date pin) is enough.

---

## 3. RPCs we actually need

### 3.1 `GetPlayerState`

Unary. Empty request. Returns `PlayerState` (`kopuz.proto`):

| Field | Role for chatbox |
| --- | --- |
| `rev` | ignore unless debugging |
| `now_ms` | daemon clock at send; pair with position |
| `phase` | engine truth (playing / paused / …) |
| `intent` | optimistic UI (loading vs committed) |
| `track` (`NowPlaying`) | `title`, `artist`, `album`, `duration_ms`, `key`, `uid` |
| `position` | `{ms, at_ms, playing}` — **anchor, not a 1 Hz ticker** |
| `queue` | optional extra (drop first when over 144) |
| `volume` | optional extra (drop early) |
| `fading` | during crossfade, display `fading.track` |
| `external` | Spotify Connect / similar; still a now-playing card |
| `error` | do not paint into chatbox |

Progress interpolation: compute `offset = local_now - now_ms` once, then `position.ms + (local_now - at_ms)` while `playing` is true. Do not wait for per-second events.

### 3.2 `Subscribe`

Server-streaming. No resume cursor. Attach **before** the snapshot:

1. `Subscribe`
2. `GetPlayerState` (and queue only if we display it)
3. Apply events. Events during the snapshot are already buffered.

`EventEnvelope` kinds we care about:

- `player_state` — full snapshot on transitions
- `position` — new anchor after seek/pause
- `resync` — mirror is junk; refetch snapshots
- `config_changed` — Discord / source flags may have flipped
- stream end — daemon died; reconnect with backoff

We do **not** need `Play` / `Pause` / queue edits for v1. nixpresence is a spectator.

### 3.3 Errors

gRPC status is the contract (`docs/api.md` table). `UNAVAILABLE` is either "source down" or "daemon gone" (local transport cause). Treat unknown codes as internal and fall back to MPRIS.

---

## 4. MPRIS on Linux

Kopuz already publishes MPRIS from `crates/player/src/systemint/linux.rs` (`mpris_server`, bus identity `"kopuz"`, well-known `org.mpris.MediaPlayer2.kopuz`).

That means **every** Linux media consumer already sees Kopuz as a normal player. nixpresence should:

1. Try Kopuz gRPC (richer: phase/intent, fading, external device, bitrate).
2. If the socket is missing, unreadable, or the stub fails proto compatibility → **MPRIS** via `zbus`.
3. On MPRIS, prefer the Kopuz player when several are playing, but allow a user override (`music.player = "kopuz" | "any" | "<bus suffix>"`).

MPRIS gives us: `PlaybackStatus`, `Metadata` (`xesam:title`, `xesam:artist`, `xesam:album`, `mpris:length`, `mpris:artUrl`), `Position`. Good enough for a chatbox line.

Do not take playback control in v1 (no PlayPause from the TUI unless we add it later).

---

## 5. Discord presence inside Kopuz

Kopuz already projects now-playing into Discord.

| Item | Value | Location |
| --- | --- | --- |
| Application ID | `1470087339639443658` | `crates/daemon/src/integrations.rs` (`DISCORD_APP_ID`) |
| Keepalive / reconnect tick | **30 seconds** | same file, `DISCORD_TICK_SECS` |
| Crate | `crates/discord-presence` (`discord_presence::Presence`) | uses `discord-rich-presence` |
| Config keys | `discord_presence` (default true), `discord_presence_paused` (default true), `discord_presence_source` (default true) | `crates/config/src/lib.rs` |
| Spawn | `spawn_discord_presence` from `crates/daemon/src/boot.rs` | event-driven + 30s `presence.tick()` |

Activity flavor: Listening, title/artist/album, timestamps, optional cover URL, optional source name.

**This is Kopuz's Discord card, not ours.** Two processes cannot both own a useful "Listening to X" presence on one Discord client without fighting. See coexistence below.

---

## 6. Recommended coexistence modes

Discord IPC is a single activity per application ID, and the user's Discord client shows one game/listening card from the last writer that "wins." Music chatbox and music presence must be coordinated.

| Mode | Kopuz `discord_presence` | nixpresence Discord | Music source | When to use |
| --- | --- | --- | --- | --- |
| **A. Kopuz owns Discord** (default if Kopuz is running) | leave on (true) | **disable** our Listening activity; we may still show a *Playing VRChat* card with a **different** app id | gRPC or MPRIS | User already likes Kopuz's cover-art Listening card |
| **B. nixpresence owns Discord** | set `discord_presence = false` in Kopuz settings | we publish Listening **or** Playing VRChat (our app id) | gRPC or MPRIS | User wants one card that mixes world + track |
| **C. Neither music card** | false | VRChat-only presence, or off | still read music for OSC | Chatbox only |
| **D. No Kopuz** | n/a | our presence if enabled | MPRIS (any player) | Spotify/mpv/etc. |

Config sketch for nixpresence:

```toml
[music]
backend = "auto"          # auto | kopuz | mpris
mpris_identity = "kopuz"  # preferred bus when several play

[discord]
enabled = true
mode = "vrchat"           # off | vrchat | listening | auto
# auto: if Kopuz gRPC is up AND kopuz discord_presence is true → vrchat-only
#       else → listening from whatever music backend we have
```

Detection of Kopuz's flag: `GetConfig` exists on the daemon (`docs/api.md` Settings). Optional. If we do not want to depend on `Config` schema drift, **mode A/B is a user toggle** and we document "turn Kopuz Discord off if you want nixpresence to show the track."

Never call `SetConfig` to flip their Discord flag for them.

---

## 7. Preferred client order

```text
                    ┌─ unix socket exists ─► tonic GetPlayerState + Subscribe
music.backend=auto ─┤
                    └─ else / RPC fail    ─► zbus MPRIS (prefer kopuz, else any Playing)
```

Implementation notes:

- Connect with tonic + unix domain (`http://[::]:0` dummy + custom connector, or `tonic::transport` unix).
- Reconnect with jittered backoff; on `resync` or stream end, resubscribe then snapshot.
- Interpolate position locally; do not poll `GetPlayerState` at 1 Hz if the stream is alive. A 5–10 s liveness poll is enough.
- Map `Phase`/`Intent` to a small enum we own (`Playing`, `Paused`, `Stopped`, `Loading`). Unknown → `Stopped` for the chatbox (hide the segment).
- Title line recipe (ours): `♪ {artist} — {title}` plus optional `[paused]`. Queue/volume/device are extra segments with high drop weight.

---

## 8. What not to do

- Do not vendor Kopuz crates into our workspace.
- Do not copy `crates/discord-presence` or reuse application id `1470087339639443658`.
- Do not expose Kopuz TCP or read `kopuzd.token`.
- Do not start `kopuzd` ourselves; the user (or Nix) owns that service.
- Do not assume the proto we snapshot today still builds against next week's daemon without a compile/runtime guard.

---

## 9. File index (clone)

| Path | Why it matters |
| --- | --- |
| `docs/api.md` | human contract, socket, attach order |
| `crates/proto/proto/kopuz.proto` | `GetPlayerState`, `Subscribe`, `PlayerState` |
| `crates/client/` | reference Rust client |
| `crates/daemon/src/integrations.rs` | Discord app id, 30 s tick, `discord_presence*` |
| `crates/discord-presence/src/lib.rs` | IPC tick / reconnect behavior |
| `crates/config/src/lib.rs` | `discord_presence` defaults |
| `crates/player/src/systemint/linux.rs` | MPRIS server, identity `kopuz` |
