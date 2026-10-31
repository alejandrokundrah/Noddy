// SPDX-License-Identifier: GPL-3.0-or-later
//! The Noddy mark (`assets/noddy/Noddy black.svg`): a long shape with a round
//! end at each corner, joined by a diagonal bridge, and a drop on each side of
//! it. It sits on an espresso tile with a warm glow. Drawn in code at any
//! pixel size: the tray, window and toast icons at runtime, and the
//! executable's icon at build time (`build.rs` includes this file), so every
//! size is sharp without bitmap assets. At 32 px and below the tile fills more
//! of the square and the mark is a little larger, as in the hand-tuned small
//! sizes.

/// Espresso tile, its glow, the honey rim and the cream mark
/// (docs/design/tokens.json, Honey palette).
const TILE: [f32; 3] = rgb(0x2C, 0x16, 0x00);
const GLOW: [f32; 3] = rgb(0x69, 0x3C, 0x00);
const RIM: [f32; 3] = rgb(0xFF, 0xB9, 0x5F);
const MARK: [f32; 3] = rgb(0xFF, 0xF8, 0xF3);

/// One of the mark's closed shapes: where it starts, then cubic Béziers as
/// (control, control, end), the last ending where the shape started.
struct Shape {
    start: (f32, f32),
    curves: &'static [[(f32, f32); 3]],
}

/// The mark's three shapes in a 100×100 design grid, as drawn in
/// `assets/noddy/icon.svg`: the bounding box is 54 wide and centered on
/// (50, 50). They don't overlap.
const SHAPES: [Shape; 3] = [
    Shape {
        start: (56.24, 64.39),
        curves: &[
            [(56.20, 63.48), (56.01, 57.88), (52.48, 53.64)],
            [(40.91, 39.71), (33.00, 45.13), (27.96, 41.82)],
            [(17.66, 35.06), (28.25, 20.19), (38.28, 26.20)],
            [(46.45, 31.10), (39.55, 36.02), (48.29, 44.96)],
            [(61.87, 58.84), (66.63, 51.32), (72.55, 55.72)],
            [(84.12, 64.34), (70.59, 80.20), (60.88, 73.39)],
            [(55.72, 69.77), (56.32, 65.31), (56.24, 64.39)],
        ],
    },
    Shape {
        start: (42.98, 63.05),
        curves: &[
            [(44.87, 80.58), (20.25, 77.87), (23.25, 63.06)],
            [(24.52, 56.78), (29.50, 58.12), (32.05, 50.51)],
            [(33.19, 47.11), (34.55, 51.28), (34.62, 51.49)],
            [(36.58, 57.52), (40.22, 55.31), (42.98, 63.05)],
        ],
    },
    Shape {
        start: (75.93, 37.55),
        curves: &[
            [(73.71, 43.77), (71.01, 41.95), (67.93, 47.77)],
            [(67.00, 49.52), (65.50, 46.66), (65.40, 46.46)],
            [(64.14, 44.06), (54.81, 39.61), (58.89, 31.88)],
            [(63.24, 23.65), (77.23, 25.75), (75.93, 37.55)],
        ],
    },
];

/// Supersampling grid per pixel (SS×SS samples) for smooth edges.
const SS: usize = 4;

/// Line segments per Bézier when flattening an outline: at most about a unit
/// long in the design grid, well under a pixel at the largest size.
const STEPS: usize = 24;

const fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32, g as f32, b as f32]
}

/// Sizes in the design grid: the small sizes are drawn bolder.
struct Geometry {
    inset: f32,
    corner: f32,
    /// The mark's size relative to the design grid, about its middle.
    scale: f32,
}

impl Geometry {
    fn for_size(size: usize) -> Self {
        if size <= 32 {
            Geometry { inset: 2.0, corner: 20.0, scale: 1.1 }
        } else {
            Geometry { inset: 4.0, corner: 22.0, scale: 1.0 }
        }
    }
}

type Edge = ((f32, f32), (f32, f32));

/// Renders the mark as straight-alpha RGBA, row by row, `size`×`size`.
pub fn render(size: usize) -> Vec<u8> {
    let g = Geometry::for_size(size);
    let unit = 100.0 / size as f32;
    let edges = outline(g.scale);
    // The rim is at least about a pixel wide, so the tile keeps an edge on dark taskbars.
    let rim = (1.5f32).max(unit * 0.6);

    let mut out = vec![0u8; size * size * 4];
    let mut crossings = Vec::new();
    let mut color = vec![[0f32; 3]; size];
    let mut covered = vec![0usize; size];
    for py in 0..size {
        color.fill([0f32; 3]);
        covered.fill(0);
        for sy in 0..SS {
            // The sample row in the design grid, and where the mark's outline crosses it.
            let y = (py as f32 + (sy as f32 + 0.5) / SS as f32) * unit;
            crossings_at(&edges, y, &mut crossings);
            // Samples run left to right, so this only moves forward: an odd
            // number of crossings to the left of a sample means it's inside.
            let mut passed = 0;
            for px in 0..size {
                for sx in 0..SS {
                    let x = (px as f32 + (sx as f32 + 0.5) / SS as f32) * unit;
                    while passed < crossings.len() && crossings[passed] < x {
                        passed += 1;
                    }
                    let edge = rounded_square_distance(x, y, g.inset, g.corner);
                    if edge > 0.0 {
                        continue;
                    }
                    covered[px] += 1;
                    let paint = if passed % 2 == 1 {
                        MARK
                    } else if -edge <= rim {
                        mix(TILE, RIM, 0.34)
                    } else {
                        // The glow fades from the middle to the tile color at 40 units.
                        let (dx, dy) = (x - 50.0, y - 50.0);
                        let t = ((dx * dx + dy * dy).sqrt() / 40.0).min(1.0);
                        mix(GLOW, TILE, t)
                    };
                    for c in 0..3 {
                        color[px][c] += paint[c];
                    }
                }
            }
        }
        for px in 0..size {
            if covered[px] == 0 {
                continue;
            }
            let i = (py * size + px) * 4;
            for c in 0..3 {
                out[i + c] = (color[px][c] / covered[px] as f32).round() as u8;
            }
            out[i + 3] = ((covered[px] as f32 / (SS * SS) as f32) * 255.0).round() as u8;
        }
    }
    out
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// How far (x, y) is outside the rounded square from `inset` to
/// `100 - inset` with corner radius `r` (negative inside).
fn rounded_square_distance(x: f32, y: f32, inset: f32, r: f32) -> f32 {
    let (lo, hi) = (inset + r, 100.0 - inset - r);
    let qx = (x.clamp(lo, hi) - x).abs();
    let qy = (y.clamp(lo, hi) - y).abs();
    if x > lo && x < hi || y > lo && y < hi {
        // Along a straight side.
        let to_x = (x - inset).min(100.0 - inset - x);
        let to_y = (y - inset).min(100.0 - inset - y);
        return -to_x.min(to_y);
    }
    (qx * qx + qy * qy).sqrt() - r
}

/// The mark's shapes, scaled about the middle of the design grid, as the
/// straight edges of closed polygons close enough to the curves to measure against.
fn outline(scale: f32) -> Vec<Edge> {
    let at = |p: (f32, f32)| (50.0 + (p.0 - 50.0) * scale, 50.0 + (p.1 - 50.0) * scale);
    let mut edges = Vec::new();
    for shape in &SHAPES {
        let mut previous = at(shape.start);
        for curve in shape.curves {
            let start = previous;
            let (b, c, d) = (at(curve[0]), at(curve[1]), at(curve[2]));
            for step in 1..=STEPS {
                let t = step as f32 / STEPS as f32;
                let u = 1.0 - t;
                let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                let next = (
                    w[0] * start.0 + w[1] * b.0 + w[2] * c.0 + w[3] * d.0,
                    w[0] * start.1 + w[1] * b.1 + w[2] * c.1 + w[3] * d.1,
                );
                edges.push((previous, next));
                previous = next;
            }
            // Land exactly on the curve's end so the shape closes without a gap.
            if let Some(last) = edges.last_mut() {
                last.1 = d;
            }
            previous = d;
        }
    }
    edges
}

/// The x positions, sorted, where the outline crosses the horizontal line at `y`.
fn crossings_at(edges: &[Edge], y: f32, out: &mut Vec<f32>) {
    out.clear();
    for &((x0, y0), (x1, y1)) in edges {
        if (y0 <= y) != (y1 <= y) {
            out.push(x0 + (y - y0) / (y1 - y0) * (x1 - x0));
        }
    }
    out.sort_by(f32::total_cmp);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(img: &[u8], size: usize, x: usize, y: usize) -> [u8; 4] {
        let i = (y * size + x) * 4;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    /// The pixel at a point of the design grid.
    fn at(img: &[u8], size: usize, x: f32, y: f32) -> [u8; 4] {
        pixel(img, size, (x / 100.0 * size as f32) as usize, (y / 100.0 * size as f32) as usize)
    }

    const CREAM: [u8; 4] = [0xFF, 0xF8, 0xF3, 255];

    #[test]
    fn corners_are_transparent_and_the_tile_is_dark() {
        let size = 64;
        let img = render(size);
        assert_eq!(img.len(), size * size * 4);
        assert_eq!(pixel(&img, size, 0, 0)[3], 0, "rounded corner");
        // Near the top edge, away from the mark: the dark tile.
        let tile = at(&img, size, 32.0, 8.0);
        assert!(tile[0] < 0x80 && tile[3] == 255, "{tile:?}");
        // Between the bottom-left drop and the bridge: still the tile, not the mark.
        let gap = at(&img, size, 50.0, 70.0);
        assert!(gap[0] < 0x80 && gap[3] == 255, "{gap:?}");
    }

    #[test]
    fn all_three_shapes_are_drawn() {
        let size = 128;
        let img = render(size);
        // The two round ends of the long shape, then the drop at the bottom left and the one at the top right.
        for (x, y) in [(33.0, 34.0), (68.0, 66.0), (33.1, 64.4), (67.0, 36.3)] {
            assert_eq!(at(&img, size, x, y), CREAM, "({x}, {y})");
        }
    }

    #[test]
    fn renders_every_tray_size() {
        for size in [16, 20, 24, 32, 48, 64, 256] {
            let img = render(size);
            assert!(img.chunks(4).any(|p| p[3] == 255), "{size}px has an opaque tile");
            assert!(
                img.chunks(4).any(|p| p[0] > 0xE0 && p[1] > 0xD0 && p[3] == 255),
                "{size}px shows the mark"
            );
        }
    }
}
