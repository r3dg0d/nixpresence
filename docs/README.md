# nixpresence docs

| Doc | Purpose |
|-----|---------|
| [architecture-decision.md](./architecture-decision.md) | Why MIT Rust, XDG, provider pipeline |
| [magicchatbox-analysis.md](./magicchatbox-analysis.md) | Concepts only — **do not copy MagicChatBox source** (proprietary SLA) |
| [kopuz-integration.md](./kopuz-integration.md) | MPRIS primary, gRPC optional, Discord coexist |
| [vrchat-osc-notes.md](./vrchat-osc-notes.md) | `/chatbox/input` s b n, 144 grapheme budget |
| [packaging.md](./packaging.md) | Flake + NixOS module (MatrixShot-style) |

**Hard rules**

- Never reuse Kopuz Discord application id `1470087339639443658`.
- Default `[discord.music] mode = "coexist"`.
- OSC ticker uses `notify = false` to avoid SFX spam.
