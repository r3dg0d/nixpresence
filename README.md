# nixpresence

NixOS-friendly **VRChat chatbox** compositor + optional **Discord Rich Presence** daemon.

Independent **MIT** implementation inspired by the *ideas* behind MagicChatBox — **not** a port
of that proprietary codebase. Music comes from **MPRIS** (Kopuz preferred by name); Kopuz gRPC
is optional best-effort.

## Quick start

```bash
cd ~/Projects/nixpresence
cargo build --release
./target/release/nixpresence config init
./target/release/nixpresence doctor
./target/release/nixpresence preview
./target/release/nixpresence test osc          # harmless msg → 127.0.0.1:9000
./target/release/nixpresence daemon --stdout   # foreground
```

Systemd user unit (after install):

```bash
systemctl --user enable --now nixpresence.service
```

## Architecture

```
Providers → shared State → Template/Compositor → Outputs (vrchat_osc, discord, stdout)
```

## Discord

Create **your own** Discord Application ID.  
**Never** use Kopuz’s id `1470087339639443658`.  
Default `[discord.music] mode = "coexist"` — do not fight Kopuz/arrpc music presence.

Rich Presence images use **asset keys** (or HTTPS URLs) from
`[discord.assets]`. Upload art under Developer Portal → your Application →
**Rich Presence → Art Assets**. Ship a NixOS snowflake as key `nixos`
(see [`assets/discord/`](./assets/discord/)).

```bash
export NIXPRESENCE_DISCORD_APP_ID=your_id   # for tests
```

## License

MIT — see [LICENSE](./LICENSE).
