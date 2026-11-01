# Design

Contents:

| Path | What it is |
|---|---|
| `tokens.json` | Single source of truth for colors (per theme and seed), type, spacing, shapes, elevation and motion. QML and Compose themes are generated from it |
| `themes/index.html` | Theme overview: Home screens, color roles, typography, components |
| `mockups/index.html` | Core flows: pairing, phone setup, Choose your power, notifications, messages, mirroring and app windows, Deck, Power settings |
| `assets/` | Shared preview stylesheet and helpers used by the mockups |

Open the HTML files in a browser and switch themes and Material You seeds at the top.

## Decision (2026-11-01)

- **Glass** is the main theme: translucent cards in near-black (dark) or near-white (light) with one
  orange accent (`#EB6030` dark, `#B23808` light), large corner radii, a white pill for the selected
  navigation item and a tile ("bento") layout on Home. Colors, shape and type live in the `glass`
  token set in `tokens.json`.
- **Ember** is the same look with solid cards, dark only.
- The other themes and color presets are gone. The older previews in `themes/` and `mockups/`
  were drawn for them and are out of date.

## Logo

The Noddy logo is in [`assets/noddy`](../../assets/noddy/README.md): `Noddy black.svg` for light
backgrounds and `Noddy white.svg` for dark ones. The app icon puts it in near-white (`#F4F4F5`) on a
near-black tile with an orange rim.

## Principles

- No traffic-light green: success, done and connected states use the theme's own color
  (the Material You primary).
- Calm first: neutral surfaces, color only where it carries meaning (state, primary action, the
  connected device).
- No sloppy decoration: every texture, shadow and annotation must survive a 9-hour workday.
- Light and dark share the same components and layout; they only change tokens.
- Fonts are bundled (SIL OFL): Figtree (UI) and Space Mono (code and numbers). No fonts from the web at runtime.
