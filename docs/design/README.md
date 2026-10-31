# Design

Contents:

| Path | What it is |
|---|---|
| `tokens.json` | Single source of truth for colors (per theme and seed), type, spacing, shapes, elevation and motion. QML and Compose themes are generated from it |
| `themes/index.html` | Theme overview: Home screens, color roles, typography, components |
| `mockups/index.html` | Core flows: pairing, phone setup, Choose your power, notifications, messages, mirroring and app windows, Deck, Power settings |
| `assets/` | Shared preview stylesheet and helpers used by the mockups |

Open the HTML files in a browser and switch themes and Material You seeds at the top.

## Decision (2026-10-31)

- **Glass** is the only theme: translucent cards over the window backdrop in the Honey colors
  (Material You tonal color roles), light and dark. It is built on the Bloom token set, which
  stays in `tokens.json` as Glass's palette, shape and type.
- **Hive** (honey with a living comb), **Graphite** (ink on paper) and the other Bloom color
  seeds are no longer offered in the apps. The older design previews in `themes/` and
  `mockups/` still show them.

## Logo

The Noddy logo is in [`assets/noddy`](../../assets/noddy/README.md): `Noddy black.svg` for light
backgrounds and `Noddy white.svg` for dark ones. The app icon puts it in cream (`#FFF8F3`) on the
Honey espresso tile.

## Principles

- No traffic-light green: success, done and connected states use the theme's own color
  (the Material You primary).
- Calm first: neutral surfaces, color only where it carries meaning (state, primary action, the
  connected device).
- No sloppy decoration: every texture, shadow and annotation must survive a 9-hour workday.
- Light and dark share the same components and layout; they only change tokens.
- Fonts are bundled (SIL OFL): Figtree (UI), Instrument Serif (Graphite display),
  Space Mono (Graphite labels), Caveat (Graphite margin notes). No fonts from the web at runtime.
