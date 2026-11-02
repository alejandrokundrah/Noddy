# Desktop control (protocol v0 extension)

> Status: **draft proposal.** Extends [remote input](remote.md) and
> [screen mirroring](mirror.md). Breaking changes are allowed until v1.
> License: CC BY 4.0.

One computer (the **viewer**) shows another computer's screen (the **host**)
and controls it with its own mouse and keyboard. The host already streams its
screen to phones as `mirror` session 0 ([mirror.md](mirror.md) §3); this
document adds what a computer needs beyond what a phone does: choosing a
monitor, an absolute pointer, and keys that go down and up.

## 1. Capabilities

| ID | Offered by | Meaning |
|---|---|---|
| `mirror.capture` | host | Streams its screen (existing) |
| `input.desktop` | host | Accepts the desktop input in §3 |
| `desktop.view` | viewer | Shows and controls another computer's screen |

A viewer offers the controls only for a host that offers both `mirror.capture`
and `input.desktop`. Both devices must allow it: the `mirroring` toggle on each
side, and `remote_input` on the host, which stays **off by default**.

## 2. Monitors

```
t = "remote.monitors"  id = n
t = "remote.monitors"  re = n   b = { monitors: [{ index: uint, name: text,
                                     x: int, y: int, width: uint, height: uint,
                                     primary: bool }] }
t = "mirror.monitor"            b = { monitor: uint }
```

- Positions and sizes are physical pixels in the host's virtual desktop.
  `index` 0 is the primary monitor.
- `mirror.start` gains `? monitor: uint` (default 0). `mirror.monitor`
  (viewer to host, not answered) switches while streaming: the host sends a new
  `config` packet and a keyframe. An index out of range is `BAD_MESSAGE`.

## 3. Input

`Input` ([remote.md](remote.md) §3) gains:

```
Input =
  | ...
  | { type: "abs",   x: float32, y: float32 }
  | { type: "keyev", code: text, down: bool }

Button = "left" | "right" | "middle" | "back" | "forward"
```

- **`abs`** places the pointer at fractions of the monitor now streaming
  (`0.0..=1.0`, left to right and top to bottom). It is datagram-eligible
  (channel tag `0x01`) like `move`. The host maps it with the monitor's origin
  and size, so it is exact on any monitor layout and at any display scale.
- **`keyev`** is a physical key going down or up. `code` is the DOM
  `KeyboardEvent.code` name (`KeyA`, `Digit1`, `ArrowLeft`, `ControlLeft`,
  `Numpad5`, `IntlBackslash`, `F13`, ...), at most 32 ASCII bytes. Sending the
  physical key means the host's keyboard layout applies, as if typed there.
  A `down` for a key already held is an auto-repeat. Text from input methods or
  dead keys uses `text` instead. Sent on the control stream as `remote.input`.
  An unknown `code` is `BAD_MESSAGE`; so is a 17th key held at once.
- **Release rules.** The host releases every held key and button when the
  session ends, when `remote_input` is turned off, and when the connection
  drops, so nothing stays stuck.
- **System keys.** Ctrl+Alt+Del can't be injected by an app in the user's
  session (it needs a system service), and the lock screen and UAC prompts run
  on a separate desktop. The viewer offers Alt+Tab, Win+D and Ctrl+Shift+Esc as
  plain sequences of `keyev`. While the host can't capture its screen (secure
  desktop), it keeps the session and retries, and the viewer says it is waiting.

## 4. Safety

- The host asks the user before each session (the existing mirror request),
  shows that it is being controlled for as long as it is, and lets its user end
  the session from the host. Unattended access is a separate per-device toggle,
  off by default, and skips only the prompt, never pairing.
- Rate limits follow [remote.md](remote.md) §4. Key repeat and fast typing
  need more headroom on control requests for `input.desktop` peers: use a burst
  of 120.
- Implementations **MUST NOT** log `code` or `text`: key codes reveal
  passwords as well as text does (v0 §11).

## 5. Notes

The shape of this design (monitor-relative absolute pointer, DOM key codes with
down and up, releasing held keys when a session ends) was informed by reading
the open-source remote desktop projects Veyndesk (AGPL-3.0) and sudo (MIT). No
source code was copied from either.
