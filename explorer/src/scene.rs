//! Backdrop and chart: parallax starfield, the system's sun, the galactic chart.

use elite::StarSystem;

use crate::fb::{Fb, H, Rgb, W, dither_levels, lerp, scale};
use crate::noise::{rand3, value3};

/// Screen pixels the nearest star layer moves per galactic chart unit.
const PAN: f32 = 4.0;

/// One background star.
pub struct Star {
    /// Horizontal position as a fraction of the screen width.
    x: f32,
    /// Vertical position as a fraction of the screen height.
    y: f32,
    /// Parallax depth: 1 is the nearest layer and moves the most.
    depth: f32,
    /// Brightness from 0.3 to 1.
    bright: f32,
}

/// `n` deterministic stars, mostly far away and faint.
pub fn starfield(n: usize) -> Vec<Star> {
    (0..n as i32)
        .map(|i| Star {
            x: rand3(i, 0, 0, 0x57A2),
            y: rand3(i, 1, 0, 0x57A2),
            depth: 0.15 + rand3(i, 2, 0, 0x57A2).powi(2) * 0.85,
            bright: 0.3 + rand3(i, 3, 0, 0x57A2) * 0.7,
        })
        .collect()
}

/// Draws the stars as seen from galactic position `cam`. `warp` (0..1)
/// stretches them into hyperspace streaks radiating from the centre.
pub fn draw_starfield(fb: &mut Fb, stars: &[Star], cam: [f32; 2], t: f32, warp: f32) {
    let (cx, cy) = (W as f32 / 2.0, H as f32 / 2.0);
    for (i, s) in stars.iter().enumerate() {
        let sx = (s.x * W as f32 - cam[0] * s.depth * PAN).rem_euclid(W as f32);
        let sy = (s.y * H as f32 - cam[1] * s.depth * PAN * 0.5).rem_euclid(H as f32);
        let twinkle = 0.85 + 0.15 * (t * 3.0 + i as f32).sin();
        let c = scale([200, 210, 255], s.bright * s.depth.sqrt() * twinkle);

        if warp > 0.0 {
            let (dx, dy) = (sx - cx, sy - cy);
            let len = warp * warp * s.depth * 3.5;
            let steps = (len * (dx.abs() + dy.abs()) * 0.5).max(1.0) as i32;
            for k in 0..=steps {
                let f = 1.0 + len * k as f32 / steps as f32;
                fb.add((cx + dx * f) as i32, (cy + dy * f) as i32, scale(c, 0.6));
            }
        } else {
            fb.set(sx as i32, sy as i32, c);
        }
    }
}

/// The system's star, peeking in from the top-left corner.
pub fn draw_sun(fb: &mut Fb, cx: f32, cy: f32, r: f32, color: Rgb, t: f32, flares: bool) {
    let reach = r * 2.2;
    let x1 = ((cx + reach) as i32).min(W - 1);
    let y1 = ((cy + reach) as i32).min(H - 1);
    for y in 0..=y1 {
        for x in 0..=x1 {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let d = (dx * dx + dy * dy).sqrt() / r;
            if d < 1.0 {
                // Limb darkening: white-hot centre, coloured rim.
                let k = dither_levels(1.0 - d * d * 0.45, 6.0, x, y);
                fb.set(x, y, scale(lerp([255, 255, 240], color, d * d), k));
                continue;
            }
            let mut glow = ((2.2 - d) / 1.2).clamp(0.0, 1.0).powi(2) * 0.55;
            if flares && d < 1.6 {
                let ang = dy.atan2(dx);
                let n = value3([ang * 5.0, t * 1.2, 0.3], 0xF1A2);
                if (d - 1.0) / 0.6 < (n - 0.5) * 2.4 {
                    fb.set(x, y, lerp(color, [255, 120, 40], 0.5));
                    continue;
                }
            }
            if glow > 0.0 {
                glow *= 0.9 + 0.1 * (t * 2.0).sin();
                fb.set_dithered(x, y, scale(color, 0.7), glow);
            }
        }
    }
}

/// Screen X of the chart's left edge (chart X 0).
pub const CHART_X: f32 = 32.0;
/// Screen Y of the chart's top edge (chart Y 0).
pub const CHART_Y: f32 = 40.0;

/// Where a system sits on the 256x128 galactic chart, in screen pixels.
pub fn chart_pos(s: &StarSystem) -> [f32; 2] {
    [CHART_X + s.x as f32, CHART_Y + s.y as f32 / 2.0]
}

/// Chart dot colour: blue-white for industrial, green for agricultural economies.
fn economy_color(economy: u8) -> Rgb {
    if economy <= 3 {
        [150, 180, 255] // industrial: cool blue-white
    } else {
        [190, 255, 150] // agricultural: green
    }
}

/// The galactic chart, faded in by `alpha` with a dither pattern.
pub fn draw_chart(fb: &mut Fb, systems: &[StarSystem], cursor: [f32; 2], alpha: f32, t: f32) {
    if alpha <= 0.0 {
        return;
    }
    let frame = [0, 170, 200];
    let (x0, y0) = (CHART_X as i32 - 2, CHART_Y as i32 - 2);
    let (x1, y1) = (x0 + 260, y0 + 132);
    for x in x0..=x1 {
        fb.set_dithered(x, y0, frame, alpha);
        fb.set_dithered(x, y1, frame, alpha);
    }
    for y in y0..=y1 {
        fb.set_dithered(x0, y, frame, alpha);
        fb.set_dithered(x1, y, frame, alpha);
    }

    for s in systems {
        let [x, y] = chart_pos(s);
        let c = economy_color(s.economy);
        fb.set_dithered(x as i32, y as i32, c, alpha);
        if s.population >= 45 {
            // Big populations get a brighter, larger dot.
            for (ox, oy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                fb.set_dithered(x as i32 + ox, y as i32 + oy, scale(c, 0.55), alpha);
            }
        }
    }

    if alpha > 0.6 {
        let (cx, cy) = (cursor[0].round() as i32, cursor[1].round() as i32);
        // 7 light years: 4 * sqrt(dx^2 + (dy/2)^2) / 10 = 7  =>  17.5 chart pixels.
        fb.circle(cx, cy, 17.5, [0, 110, 140]);
        let blink = if (t * 3.0).fract() < 0.7 {
            [255, 255, 80]
        } else {
            [180, 180, 60]
        };
        for k in 3..7 {
            fb.set(cx + k, cy, blink);
            fb.set(cx - k, cy, blink);
            fb.set(cx, cy + k, blink);
            fb.set(cx, cy - k, blink);
        }
    }
}

/// Tests for the starfield, sun and chart.
#[cfg(test)]
mod tests {
    use super::*;

    /// Chart coordinates map 256x256 galaxy space to 256x128 screen pixels.
    #[test]
    fn chart_pos_halves_y() {
        let lave = &elite::galaxy(0)[7];
        assert_eq!(chart_pos(lave), [CHART_X + 20.0, CHART_Y + 86.5]);
    }

    /// Stars are deterministic and stay within their documented ranges.
    #[test]
    fn starfield_is_deterministic_and_in_range() {
        let a = starfield(100);
        let b = starfield(100);
        for (s, t) in a.iter().zip(&b) {
            assert_eq!((s.x, s.y, s.depth), (t.x, t.y, t.depth));
            assert!((0.0..=1.0).contains(&s.x) && (0.0..=1.0).contains(&s.y));
            assert!((0.15..=1.0).contains(&s.depth) && (0.3..=1.0).contains(&s.bright));
        }
    }

    /// Moving the camera shifts the stars; the same camera draws the same frame.
    #[test]
    fn starfield_parallax() {
        let stars = starfield(60);
        let snap = |cam| {
            let mut fb = Fb::new();
            draw_starfield(&mut fb, &stars, cam, 0.0, 0.0);
            fb.pixels
        };
        assert_eq!(snap([0.0, 0.0]), snap([0.0, 0.0]));
        assert_ne!(snap([0.0, 0.0]), snap([10.0, 0.0]));
    }

    /// Warp streaks light up many more pixels than plain stars.
    #[test]
    fn warp_streaks_stretch_stars() {
        let stars = starfield(60);
        let lit = |warp| {
            let mut fb = Fb::new();
            draw_starfield(&mut fb, &stars, [0.0, 0.0], 0.0, warp);
            fb.pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[..3] != [0, 0, 0])
                .count()
        };
        assert!(lit(1.0) > lit(0.0) * 3);
    }

    /// The sun's centre is bright and its glow fades out.
    #[test]
    fn sun_is_bright_in_the_middle() {
        let mut fb = Fb::new();
        draw_sun(&mut fb, 100.0, 100.0, 20.0, [255, 240, 180], 0.0, false);
        let lum = |c: Rgb| c.iter().map(|&v| v as u32).sum::<u32>();
        assert!(lum(fb.get(100, 100)) > 600);
        assert_eq!(fb.get(160, 160), [0; 3]);
    }

    /// A fully faded chart draws nothing; a visible one draws its frame.
    #[test]
    fn chart_respects_alpha() {
        let systems = elite::galaxy(0);
        let mut fb = Fb::new();
        draw_chart(&mut fb, &systems, [100.0, 100.0], 0.0, 0.0);
        assert!(
            fb.pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[..3] == [0, 0, 0])
        );
        draw_chart(&mut fb, &systems, [100.0, 100.0], 1.0, 0.0);
        assert_eq!(
            fb.get(CHART_X as i32 - 2, CHART_Y as i32 - 2),
            [0, 170, 200]
        );
    }

    /// Industrial and agricultural systems get different chart colours.
    #[test]
    fn economy_colours_split_by_type() {
        assert_eq!(economy_color(0), economy_color(3));
        assert_eq!(economy_color(4), economy_color(7));
        assert_ne!(economy_color(3), economy_color(4));
    }
}
