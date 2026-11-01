// SPDX-License-Identifier: GPL-3.0-or-later
// Generated from docs/design/tokens.json by `cargo xtask tokens`. Do not edit.
pragma Singleton
import QtQuick

QtObject {
    readonly property var data: ({
      "elevation": {
        "bloom": {
          "level0": "none",
          "level1": "0 1px 2px rgba(0,0,0,.06)",
          "window": "0 1px 2px rgba(0,0,0,.06), 0 24px 60px -20px rgba(0,0,0,.18)"
        }
      },
      "icons": {
        "set": "Material Symbols Rounded (Apache-2.0)",
        "size": 20,
        "stroke": "outlined, 400 weight; filled when selected"
      },
      "layout": {
        "desktop": {
          "contentPadding": 22,
          "gutter": 16,
          "minWindow": [
            880,
            600
          ],
          "railWidth": 80,
          "topBarHeight": 52
        },
        "phone": {
          "gutter": 10,
          "screenPadding": 16,
          "tabBarHeight": 64
        }
      },
      "motion": {
        "description": "Springs everywhere; durations are only for fades. Respect reduced-motion: replace movement with 120 ms fades.",
        "fade": {
          "fast": 120,
          "normal": 200
        },
        "hoverLift": 2,
        "pressScale": 0.97,
        "spring": {
          "gentle": {
            "damping": 26,
            "stiffness": 220,
            "use": "page and panel transitions"
          },
          "snappy": {
            "damping": 34,
            "stiffness": 620,
            "use": "toggles, chips, buttons, hover lift"
          },
          "standard": {
            "damping": 30,
            "stiffness": 380,
            "use": "cards, sheets, expanding a notification into a conversation"
          }
        }
      },
      "spacing": {
        "scale": [
          0,
          2,
          4,
          6,
          8,
          10,
          12,
          16,
          20,
          24,
          32,
          40,
          48,
          64
        ],
        "unit": 4
      },
      "themes": {
        "ember": {
          "description": "Dark only. Near-black cards on black with one orange accent, a white pill for the selected item, large corner radii and a bento grid on Home. The look of the Ember theme.",
          "dynamicColor": false,
          "fonts": {
            "display": "Figtree",
            "label": "Figtree",
            "mono": "Space Mono",
            "ui": "Figtree"
          },
          "indicator": {
            "color": "#F4F4F5",
            "content": "#111113"
          },
          "label": {
            "case": "none",
            "size": 12.5,
            "tracking": 0,
            "weight": 600
          },
          "shape": {
            "full": 999,
            "lg": 24,
            "md": 16,
            "sm": 12,
            "xl": 32,
            "xs": 8
          },
          "variants": {
            "dark": {
              "error": "#FF8A7A",
              "onError": "#4A0A02",
              "onPrimary": "#2A0D00",
              "onPrimaryContainer": "#FFDBCE",
              "onSecondaryContainer": "#EDEDEF",
              "onSurface": "#F4F4F5",
              "onSurfaceVariant": "#A1A1A6",
              "outline": "#6E6E74",
              "outlineVariant": "#2A2A2D",
              "primary": "#EB6030",
              "primaryContainer": "#5A2412",
              "secondaryContainer": "#2C2C30",
              "surface": "#0A0A0B",
              "surfaceContainer": "#141415",
              "surfaceContainerHigh": "#1B1B1D",
              "surfaceContainerHighest": "#242427",
              "surfaceContainerLow": "#0E0E0F",
              "warning": "#F2C14E"
            }
          }
        },
        "glass": {
          "defaultSeed": "orange",
          "description": "Translucent near-black (or near-white) cards with one orange accent, large corner radii. The look of the Glass theme.",
          "dynamicColor": true,
          "fonts": {
            "display": "Figtree",
            "label": "Figtree",
            "mono": "Space Mono",
            "ui": "Figtree"
          },
          "label": {
            "case": "none",
            "size": 12.5,
            "tracking": 0,
            "weight": 600
          },
          "seeds": {
            "orange": {
              "dark": {
                "onPrimary": "#2A0D00",
                "onPrimaryContainer": "#FFDBCE",
                "onSecondaryContainer": "#EDEDEF",
                "onSurface": "#F4F4F5",
                "onSurfaceVariant": "#A1A1A6",
                "outline": "#6E6E74",
                "outlineVariant": "#2A2A2D",
                "primary": "#EB6030",
                "primaryContainer": "#5A2412",
                "secondaryContainer": "#2C2C30",
                "surface": "#0A0A0B",
                "surfaceContainer": "#141415",
                "surfaceContainerHigh": "#1B1B1D",
                "surfaceContainerHighest": "#242427",
                "surfaceContainerLow": "#0E0E0F"
              },
              "light": {
                "onPrimary": "#FFFFFF",
                "onPrimaryContainer": "#3A0F00",
                "onSecondaryContainer": "#1B1B1E",
                "onSurface": "#18181B",
                "onSurfaceVariant": "#52525B",
                "outline": "#8A8A92",
                "outlineVariant": "#D4D4D8",
                "primary": "#B23808",
                "primaryContainer": "#FFDBCE",
                "secondaryContainer": "#E9E9EC",
                "surface": "#FAFAFA",
                "surfaceContainer": "#EEEEF0",
                "surfaceContainerHigh": "#E8E8EA",
                "surfaceContainerHighest": "#E2E2E5",
                "surfaceContainerLow": "#F4F4F5"
              },
              "seed": "#E85C29"
            }
          },
          "shape": {
            "full": 999,
            "lg": 24,
            "md": 16,
            "sm": 12,
            "xl": 32,
            "xs": 8
          },
          "status": {
            "dark": {
              "error": "#FFB4AB",
              "onError": "#690005",
              "success": "{primary}",
              "warning": "#F2C14E"
            },
            "light": {
              "error": "#BA1A1A",
              "onError": "#FFFFFF",
              "success": "{primary}",
              "warning": "#7D5200"
            }
          }
        }
      },
      "typography": {
        "body": {
          "lineHeight": 1.5,
          "size": 14,
          "tracking": 0,
          "weight": 400
        },
        "bodySmall": {
          "lineHeight": 1.45,
          "size": 12.5,
          "tracking": 0,
          "weight": 400
        },
        "caption": {
          "lineHeight": 1.3,
          "size": 11,
          "tracking": 0,
          "weight": 500
        },
        "code": {
          "font": "mono",
          "lineHeight": 1.3,
          "size": 13,
          "tracking": 0.14,
          "weight": 700
        },
        "displayLarge": {
          "lineHeight": 1.05,
          "size": 44,
          "tracking": -0.025,
          "weight": 650
        },
        "displaySmall": {
          "lineHeight": 1.1,
          "size": 30,
          "tracking": -0.02,
          "weight": 650
        },
        "headline": {
          "lineHeight": 1.2,
          "size": 22,
          "tracking": -0.015,
          "weight": 650
        },
        "label": {
          "lineHeight": 1.3,
          "size": 12.5,
          "tracking": 0,
          "weight": 600
        },
        "title": {
          "lineHeight": 1.35,
          "size": 16,
          "tracking": 0,
          "weight": 600
        }
      }
    })
}
