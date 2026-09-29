# Discord Rich Presence art assets

Upload these images in the [Discord Developer Portal](https://discord.com/developers/applications)
for **your** nixpresence application (not Kopuz’s):

**Application → Rich Presence → Art Assets**

`large_image` / `small_image` in config are **asset keys** (the name you give
when uploading) or HTTPS image URLs. The `discord-rich-presence` crate accepts
both.

## Required

| Asset key   | File                         | Use                          |
|-------------|------------------------------|------------------------------|
| `nixos`     | [`nixos.png`](./nixos.png)   | Large image (NixOS snowflake) |

`nixos.png` is a 1024×1024 RGBA render of the official
[NixOS snowflake](https://github.com/NixOS/nixos-artwork/blob/master/logo/nix-snowflake-colours.svg)
(`nix-snowflake-colours.svg` is also kept here for re-export).

## Recommended (small image)

| Asset key      | Suggested art                         | When shown                                      |
|----------------|---------------------------------------|-------------------------------------------------|
| `vrchat`       | VRChat logo (your own / fair use)     | VRChat process detected                         |
| `kopuz`        | Kopuz / music icon                    | Preferred music player active (coexist-friendly)|
| `equibop`      | Equibop / Discord client icon         | Optional client badge via `map` / override      |
| `nixpresence`  | Generic nixpresence / snowflake mark  | Default small image when nothing else matches   |

Small-image resolution order (see `src/outputs/discord.rs`):

1. `[discord.assets].small_image` override (if set)
2. VRChat running → `map.vrchat`
3. Preferred music player (`prefer_players`) → `map.kopuz` / normalized player name
4. `map.default` — omit small image if unset/empty

Missing keys simply fail to render that image in Discord; presence text still works.
