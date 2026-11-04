//! Backdrop and chart: parallax starfield, the system's sun, the galactic chart.

use elite::StarSystem;

use crate::fb::{Fb, H, Rgb, W, dither_levels, lerp, scale};
use crate::noise::{rand3, value3};

/// Screen pixels the nearest star layer moves per galactic chart unit.
const PAN: f32 = 4.0;

pub struct Star {
    x: f32,
    y: f32,
    depth: f32,
    bright: f32,
}

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

pub const CHART_X: f32 = 32.0;
pub const CHART_Y: f32 = 40.0;

/// Where a system sits on the 256x128 galactic chart, in screen pixels.
pub fn chart_pos(s: &StarSystem) -> [f32; 2] {
    [CHART_X + s.x as f32, CHART_Y + s.y as f32 / 2.0]
}

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
