# The Noddy logo

A long shape with a round end at each corner, joined by a diagonal bridge,
and a drop on each side of it.

| File | What it is |
|---|---|
| `Noddy black.svg` | The logo in black, for light backgrounds |
| `Noddy white.svg` | The logo in white, for dark backgrounds |
| `icon.svg` | The full-color app icon: the logo in cream on an espresso tile with a warm glow and a honey rim |
| `icon-flat.svg` | The same without the glow and highlights, for flat contexts |
| `icon-mono.svg` | The logo alone in white, for one-color uses (notifications, themed icons) |
| `icon-512.png` | `icon.svg` at 512 × 512 |

The apps draw the logo themselves from the same geometry:
`desktop/app/src/mark.rs` (the Windows icon, tray and notifications at every
size), `Icons.mark` in `desktop/app/qml/Icons.qml` (the logo in the window)
and `android/app/src/main/res/drawable/ic_launcher_*.xml` and
`ic_notification.xml`.

Colors (Honey palette, `docs/design/tokens.json`): tile `#2C1600`, glow
`#693C00`, logo `#FFF8F3`, rim `#FFB95F`.

The name and the logo are covered by the [trademark policy](../../TRADEMARKS.md).
