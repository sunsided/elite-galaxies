//! A 320x200 RGBA software framebuffer with ordered dithering and 8x8 text.

use font8x8::legacy::BASIC_LEGACY;

/// Framebuffer width in pixels.
pub const W: i32 = 320;
/// Framebuffer height in pixels.
pub const H: i32 = 200;

/// An sRGB colour as red, green and blue bytes.
pub type Rgb = [u8; 3];

/// 4x4 Bayer matrix, the classic ordered-dither pattern.
const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Dither threshold in `(0, 1)` for pixel `(x, y)`.
pub fn bayer(x: i32, y: i32) -> f32 {
    (BAYER4[(y & 3) as usize][(x & 3) as usize] as f32 + 0.5) / 16.0
}

/// Quantizes `v` (0..=1) to `levels` steps, dithering the remainder.
pub fn dither_levels(v: f32, levels: f32, x: i32, y: i32) -> f32 {
    let f = v.clamp(0.0, 1.0) * levels;
    let base = f.floor();
    let step = if f - base > bayer(x, y) {
        base + 1.0
    } else {
        base
    };
    (step / levels).min(1.0)
}

/// Multiplies every channel of `c` by `k`, saturating at 0 and 255.
pub fn scale(c: Rgb, k: f32) -> Rgb {
    c.map(|v| (v as f32 * k).clamp(0.0, 255.0) as u8)
}

/// Linear blend from `a` to `b`, with `t` clamped to `0..=1`.
pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8)
}

/// A W x H RGBA8 pixel buffer, row-major from the top-left corner.
pub struct Fb {
    /// RGBA bytes, four per pixel, ready for `update_texture_rgba`.
    pub pixels: Vec<u8>,
}

impl Fb {
    /// A black, fully opaque buffer.
    pub fn new() -> Self {
        Self {
            pixels: [0, 0, 0, 255].repeat((W * H) as usize),
        }
    }

    /// Fills the whole buffer with `c`.
    pub fn clear(&mut self, c: Rgb) {
        for px in self.pixels.as_chunks_mut::<4>().0 {
            *px = [c[0], c[1], c[2], 255];
        }
    }

    #[inline]
    /// Sets pixel `(x, y)` to `c`; coordinates outside the buffer are ignored.
    pub fn set(&mut self, x: i32, y: i32, c: Rgb) {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            let i = ((y * W + x) * 4) as usize;
            self.pixels[i..i + 3].copy_from_slice(&c);
        }
    }

    #[inline]
    /// Reads pixel `(x, y)`; returns black outside the buffer.
    pub fn get(&self, x: i32, y: i32) -> Rgb {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            let i = ((y * W + x) * 4) as usize;
            [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2]]
        } else {
            [0; 3]
        }
    }

    /// Sets the pixel if `alpha` beats the dither threshold: a pixel-art fade.
    #[inline]
    pub fn set_dithered(&mut self, x: i32, y: i32, c: Rgb, alpha: f32) {
        if alpha > bayer(x, y) {
            self.set(x, y, c);
        }
    }

    /// Adds light, e.g. for glows and star streaks.
    pub fn add(&mut self, x: i32, y: i32, c: Rgb) {
        let o = self.get(x, y);
        self.set(x, y, [0, 1, 2].map(|i| o[i].saturating_add(c[i])));
    }

    /// Dims a rectangle by darkening every other pixel: a cheap translucent panel.
    pub fn shade_rect(&mut self, x: i32, y: i32, w: i32, h: i32, k: f32) {
        for yy in y..y + h {
            for xx in x..x + w {
                let c = self.get(xx, yy);
                self.set(xx, yy, scale(c, k));
            }
        }
    }

    /// Draws a one-pixel circle outline of radius `r` around `(cx, cy)`.
    pub fn circle(&mut self, cx: i32, cy: i32, r: f32, c: Rgb) {
        let steps = (r * 6.0).max(16.0) as i32;
        for i in 0..steps {
            let a = i as f32 / steps as f32 * std::f32::consts::TAU;
            self.set(
                cx + (a.cos() * r).round() as i32,
                cy + (a.sin() * r).round() as i32,
                c,
            );
        }
    }

    /// Draws ASCII text with the 8x8 BBC-style font; returns the x after the text.
    pub fn text(&mut self, x: i32, y: i32, s: &str, c: Rgb) -> i32 {
        let mut cx = x;
        for ch in s.chars() {
            let glyph = BASIC_LEGACY.get(ch as usize).copied().unwrap_or([0; 8]);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        self.set(cx + col, y + row as i32, c);
                    }
                }
            }
            cx += 8;
        }
        cx
    }

    /// Text with a one-pixel drop shadow, readable over planets and stars.
    pub fn text_shadow(&mut self, x: i32, y: i32, s: &str, c: Rgb) -> i32 {
        self.text(x + 1, y + 1, s, [0, 0, 0]);
        self.text(x, y, s, c)
    }
}

/// Greedy word wrap to `cols` characters.
pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > cols {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Tests for pixel access, dithering, text and word wrap.
#[cfg(test)]
mod tests {
    use super::*;

    /// Word wrap stays within the width and loses no words.
    #[test]
    fn wrap_respects_width_and_keeps_words() {
        let lines = wrap(
            "Lave is most famous for its vast rain forests and the Lavian tree grub.",
            20,
        );
        assert!(lines.iter().all(|l| l.len() <= 20), "{lines:?}");
        assert_eq!(
            lines.join(" "),
            "Lave is most famous for its vast rain forests and the Lavian tree grub."
        );
    }

    /// 0 and 1 come out exactly at every dither position.
    #[test]
    fn dither_levels_hits_extremes() {
        for (x, y) in [(0, 0), (1, 2), (3, 3)] {
            assert_eq!(dither_levels(0.0, 4.0, x, y), 0.0);
            assert_eq!(dither_levels(1.0, 4.0, x, y), 1.0);
        }
    }

    /// Over one 4x4 tile the dithered mean equals the input.
    #[test]
    fn dither_average_matches_input() {
        // Over one 4x4 tile, the dithered mean equals the input value.
        let v = 0.3;
        let mean: f32 = (0..16)
            .map(|i| dither_levels(v, 1.0, i % 4, i / 4))
            .sum::<f32>()
            / 16.0;
        assert!((mean - v).abs() < 1.0 / 16.0 + 1e-6, "{mean}");
    }

    /// Text partly outside the buffer is clipped without panicking.
    #[test]
    fn text_draws_inside_buffer_and_ignores_clipping() {
        let mut fb = Fb::new();
        let end = fb.text(-4, -4, "A", [255, 255, 255]);
        assert_eq!(end, 4);
        fb.text(W - 4, H - 4, "WW", [255, 255, 255]);
    }

    /// Writes outside the buffer are ignored and reads there return black.
    #[test]
    fn set_and_get_clip_to_the_buffer() {
        let mut fb = Fb::new();
        fb.set(5, 7, [1, 2, 3]);
        assert_eq!(fb.get(5, 7), [1, 2, 3]);
        fb.set(-1, 0, [9; 3]);
        fb.set(W, H, [9; 3]);
        assert_eq!(fb.get(-1, 0), [0; 3]);
        assert_eq!(fb.get(W, 0), [0; 3]);
        assert!(fb.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    }

    /// Clearing fills every pixel and keeps alpha opaque.
    #[test]
    fn clear_fills_everything() {
        let mut fb = Fb::new();
        fb.clear([10, 20, 30]);
        assert!(
            fb.pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [10, 20, 30, 255])
        );
    }

    /// `scale` saturates and `lerp` clamps its parameter.
    #[test]
    fn colour_helpers() {
        assert_eq!(scale([100, 200, 50], 2.0), [200, 255, 100]);
        assert_eq!(scale([100, 200, 50], 0.0), [0, 0, 0]);
        assert_eq!(lerp([0, 0, 0], [200, 100, 50], 0.5), [100, 50, 25]);
        assert_eq!(lerp([0, 0, 0], [200, 100, 50], 7.0), [200, 100, 50]);
        assert_eq!(lerp([0, 0, 0], [200, 100, 50], -1.0), [0, 0, 0]);
    }

    /// Dithered fades draw nothing at 0, everything at 1, about half at 0.5.
    #[test]
    fn set_dithered_follows_alpha() {
        let count = |alpha: f32| {
            let mut fb = Fb::new();
            for y in 0..4 {
                for x in 0..4 {
                    fb.set_dithered(x, y, [255; 3], alpha);
                }
            }
            (0..16).filter(|i| fb.get(i % 4, i / 4) == [255; 3]).count()
        };
        assert_eq!(count(0.0), 0);
        assert_eq!(count(1.0), 16);
        assert_eq!(count(0.5), 8);
    }

    /// Bayer thresholds cover all sixteen levels strictly inside `(0, 1)`.
    #[test]
    fn bayer_thresholds_are_distinct() {
        let mut t: Vec<f32> = (0..16).map(|i| bayer(i % 4, i / 4)).collect();
        t.sort_by(f32::total_cmp);
        t.dedup();
        assert_eq!(t.len(), 16);
        assert!(t[0] > 0.0 && t[15] < 1.0);
    }

    /// Additive light saturates instead of wrapping.
    #[test]
    fn add_saturates() {
        let mut fb = Fb::new();
        fb.set(1, 1, [200, 10, 0]);
        fb.add(1, 1, [100, 10, 0]);
        assert_eq!(fb.get(1, 1), [255, 20, 0]);
    }

    /// Shading darkens only the given rectangle.
    #[test]
    fn shade_rect_darkens_inside_only() {
        let mut fb = Fb::new();
        fb.clear([200; 3]);
        fb.shade_rect(10, 10, 5, 5, 0.5);
        assert_eq!(fb.get(12, 12), [100; 3]);
        assert_eq!(fb.get(9, 12), [200; 3]);
        assert_eq!(fb.get(15, 12), [200; 3]);
    }

    /// A circle's points sit at the requested radius.
    #[test]
    fn circle_lies_on_its_radius() {
        let mut fb = Fb::new();
        fb.circle(100, 100, 10.0, [255; 3]);
        assert_eq!(fb.get(110, 100), [255; 3]);
        assert_eq!(fb.get(100, 90), [255; 3]);
        assert_eq!(fb.get(100, 100), [0; 3]);
    }

    /// Text advances eight pixels per character and actually sets pixels.
    #[test]
    fn text_advances_and_draws() {
        let mut fb = Fb::new();
        assert_eq!(fb.text(10, 10, "LAVE", [255; 3]), 42);
        let lit = (10..42)
            .flat_map(|x| (10..18).map(move |y| (x, y)))
            .filter(|&(x, y)| fb.get(x, y) == [255; 3])
            .count();
        assert!(lit > 20, "{lit}");
    }

    /// Wrapping keeps long words whole and handles empty input.
    #[test]
    fn wrap_edge_cases() {
        assert!(wrap("", 10).is_empty());
        assert_eq!(
            wrap("Supercalifragilistic word", 5),
            ["Supercalifragilistic", "word"]
        );
    }
}
