# The Noddy logo

A long shape with a round end at each corner, joined by a diagonal bridge,
and a drop on each side of it.

| File | What it is |
|---|---|
| `Noddy black.svg` | The logo in black, for light backgrounds |
| `Noddy white.svg` | The logo in white, for dark backgrounds |
| `icon.svg` | The full-color app icon: the logo in near-white on a near-black tile with a warm glow and an orange rim |
| `icon-flat.svg` | The same without the glow and highlights, for flat contexts |
| `icon-mono.svg` | The logo alone in white, for one-color uses (notifications, themed icons) |
| `icon-512.png` | `icon.svg` at 512 × 512 |

The apps draw the logo themselves from the same geometry:
`desktop/app/src/mark.rs` (the Windows icon, tray and notifications at every
size), `Icons.mark` in `desktop/app/qml/Icons.qml` (the logo in the window)
and `android/app/src/main/res/drawable/ic_launcher_*.xml` and
`ic_notification.xml`.

Colors (Glass palette, `docs/design/tokens.json`): tile `#141415`, glow
`#3A1A0E`, logo `#F4F4F5`, rim `#EB6030`.

The name and the logo are covered by the [trademark policy](../../TRADEMARKS.md).
