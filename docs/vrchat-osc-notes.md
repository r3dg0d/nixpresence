# VRChat OSC chatbox notes

**Status:** wire format and Linux/Proton facts for `nixpresence`.  
**Primary docs:**

- https://docs.vrchat.com/docs/osc-overview
- https://docs.vrchat.com/docs/osc-as-input-controller (Chatbox section)
- Community wiki (ports + chatbox table): https://wiki.vrchat.com/wiki/OSC
- Community repo notes: https://github.com/vrchat-community/osc (issue #152 documents the notify bool)

We send **into** VRChat. We do not need OSCQuery for v1.

---

## 1. Enable OSC

In VRChat: Action Menu → Options → OSC → Enabled.

If OSC is off, UDP packets are dropped on the floor. There is no handshake.

---

## 2. Ports

From the official overview:

| Direction | Default UDP port | Who binds |
| --- | --- | --- |
| App → VRChat | **9000** | VRChat listens |
| VRChat → App | **9001** | VRChat sends (avatar params, etc.) |

Launch option:

```text
--osc=inPort:senderIP:outPort
```

Defaults reproduced:

```text
--osc=9000:127.0.0.1:9001
```

`senderIP` is where **VRChat sends its outbound** traffic, not where we send. For a same-machine Linux tool, leave it `127.0.0.1`.

v1 nixpresence: send to `127.0.0.1:9000`. Bind `9001` only if we later want avatar/parameters or confirmation. Make both configurable in TOML.

---

## 3. `/chatbox/input`

Official signature (docs.vrchat.com, "OSC as Input Controller"):

```text
/chatbox/input    s   b   n
```

| Argument | OSC type | Meaning |
| --- | --- | --- |
| 1. text | string (`s`) | UTF-8 chatbox content |
| 2. send | bool (`T`/`F`) | `true`: send immediately, skip the keyboard. `false`: open the keyboard prefilled |
| 3. notification | bool (`T`/`F`) | play the chatbox "message complete" SFX. Optional; **defaults to true** if omitted |

Limits (same page):

- **144 characters** maximum
- **9 lines** maximum, counting explicit newlines **and** word wrap

Community wiki types the address as `,sTT`.

For a status line that refreshes often:

- arg2 = `true` (auto-send)
- arg3 = `false` (do **not** ding every tick)

For a user-typed message from the TUI:

- arg2 = `true` if they hit send, `false` if they want the in-game keyboard
- arg3 = `true` (or user pref)

Empty string + send `true` + notify `false` clears the chatbox.

### `/chatbox/typing`

```text
/chatbox/typing    b
```

`true` shows the `...` indicator. Useful while the user is composing in our TUI; turn it off on send. Not needed for passive status ticks.

---

## 4. Encoding the packet (rosc)

Address: `/chatbox/input`  
Args: `OscType::String(text)`, `OscType::Bool(true)`, `OscType::Bool(false)`  
Transport: UDP, one datagram per message.

Count **Unicode grapheme clusters** (or at least UTF-16-ish display width), not raw bytes, when enforcing 144. VRChat's docs say "144 characters" and "UTF-8 text." Grapheme clipping with `unicode-segmentation` is the safe default; never split a surrogate / combining mark. If a user pastes ZWJ emoji, drop or replace rather than blowing the budget.

Newlines are allowed (`\n`). A prefix / separator mode that uses enters must still stay ≤ 9 lines.

---

## 5. Rate and keep-alive

Official troubleshooting (community issues) mentions a **rate limit** on `/chatbox/input`. Burst-sending every sensor tick will drop messages.

Practical policy for nixpresence (independent; not a port of anyone's timer):

- Send when the composed string **changes**.
- If unchanged, resend every **10–15 s** so the bubble does not expire.
- Never send faster than ~2 Hz even when the string is changing (lyrics).
- Always send notify=`false` on automatic ticks.

---

## 6. Proton / Linux localhost

VRChat on Linux is a Windows build under **Steam Proton / Wine**. Proton uses the **host network stack**. A native Linux UDP send to `127.0.0.1:9000` reaches the game process. There is no extra NAT, and we do **not** send into a Windows-only named pipe.

Rules of thumb:

1. **Use `127.0.0.1`, never `localhost`.** `localhost` often resolves to `::1`. VRChat/Proton commonly binds IPv4 only. A v6 send looks like success and arrives nowhere.
2. Bind our sockets IPv4 (`UdpSocket::bind("127.0.0.1:0")` then `send_to(..., "127.0.0.1:9000")`).
3. If the user must override, Steam launch options: `--osc=9000:127.0.0.1:9001`.
4. **Flatpak / sandboxed Steam:** the tool and the game must share a network namespace. A host-native `nixpresence` talking to Flatpak VRChat may need `--share=network` or a host Steam install.
5. **VPN / "requested address is not valid in this context":** VRChat failed to bind OSC because the VPN ate interfaces. Workaround used by players: start the game (and OSC) before the VPN, or pass a LAN IP in `--osc=`, or split-tunnel the game. Document this; do not try to "fix" their VPN.
6. **OSCQuery / mDNS / Avahi:** Proton + VRChat advertising is flaky. v1 hardcodes 9000/9001. Do not require Avahi.
7. **Multiple OSC clients:** they all send to the same 9000. Last writer wins the chatbox. Two chatbox composers (Windows MagicChatBox in the prefix **and** nixpresence) will fight. Pick one.
8. **Log path if we later parse radar:**  
   `~/.local/share/Steam/steamapps/compatdata/<VRChat appid>/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat/`  
   App id is commonly `438100`. Not needed for v1 chatbox send.

---

## 7. Related addresses (out of v1, listed so we do not invent them)

| Address | Use |
| --- | --- |
| `/input/Voice` | mute toggle / PTT (int 0/1) — do not touch unless the user asks |
| `/avatar/parameters/*` | avatar floats/bools |
| `/tracking/vrsystem/*` | tracking |

MagicChatBox also offers extra outbound ports (9002/9003) for chaining tools. We can expose `extra_targets = ["127.0.0.1:9002"]` later; not required.

---

## 8. Minimal send (illustrative)

```text
UDP 127.0.0.1:9000
  /chatbox/input
    "♪ Ado — Show  ┆  GPU 62°C"
    true
    false
```

Checklist: OSC enabled in-game, IPv4 loopback, 144 graphemes, notify off, send-on-change.
