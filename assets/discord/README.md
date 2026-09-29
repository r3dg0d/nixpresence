# Discord Rich Presence art assets

Upload these images in the [Discord Developer Portal](https://discord.com/developers/applications)
for **your** nixpresence application (not Kopuz’s):

**Application → Rich Presence → Art Assets**

`large_image` / `small_image` in config are **HTTPS image URLs** (preferred when
`prefer_https = true`) or **asset keys** (the name you give when uploading). The
`discord-rich-presence` crate accepts both.

## Localhost / file:// caveat

Discord fetches Rich Presence images from **their** side (CDN / media proxy).
A tiny loopback HTTP server (`http://127.0.0.1:<port>/icon/…`) or a `file://`
MPRIS `artUrl` is almost never reachable from Discord’s infrastructure.

`serve_local_art` defaults to **false** and does not start a loopback server
until it is verified that Discord can use such URLs. Prefer:

1. Public **HTTPS** album art (`mpris:artUrl`)
2. Configurable **HTTPS** icons in `[discord.assets.map]`
3. Portal **asset keys** you uploaded

## Required

| Asset key   | File                         | Use                          |
|-------------|------------------------------|------------------------------|
| `nixos`     | [`nixos.png`](./nixos.png)   | Large image (NixOS snowflake) |

`nixos.png` is a 1024×1024 RGBA render of the official
[NixOS snowflake](https://github.com/NixOS/nixos-artwork/blob/master/logo/nix-snowflake-colours.svg)
(`nix-snowflake-colours.svg` is also kept here for re-export).

Set `large_image = "https://…"` if you prefer a public URL over the portal key.

## Config flags

```toml
[discord.assets]
large_image = "nixos"        # or https://…
large_text = "NixOS"
prefer_https = true
serve_local_art = false
```

## Recommended (small image)

| Logical key / class | Suggested value                         | When shown                                      |
|---------------------|-----------------------------------------|-------------------------------------------------|
| `vrchat`            | portal key or HTTPS                     | VRChat process / focused class                  |
| `kopuz`             | portal key or HTTPS                     | Preferred music player (when we own music RP)   |
| `firefox` / `chrome` / `steam` / … | HTTPS (Simple Icons defaults) | Focused Wayland/X11 window class           |
| `nixpresence`       | portal key `default`                    | Fallback small image                            |

Small-image resolution order (see `src/outputs/discord.rs`):

1. `[discord.assets].small_image` override (if set)
2. MPRIS `mpris:artUrl` when `https://` (or `http://` upgraded) **and** music RP
   is ours — **skipped** in `discord.music = coexist` when a preferred player
   (e.g. Kopuz) owns music presence; focused-app small image still shows
3. Focused window class (Hyprland `hyprctl activewindow -j` / X11) → `map` /
   class aliases with HTTPS defaults
4. VRChat running → `map.vrchat`
5. Preferred music player → `map.kopuz` / normalized player name
6. `map.default` — omit small image if unset/empty

Missing keys simply fail to render that image in Discord; presence text still works.
