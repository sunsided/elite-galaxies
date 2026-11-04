//! Seeded 3D value noise. Sampling it on the unit sphere gives seamless planets.

/// Integer hash of a lattice point (a murmur-style finalizer).
pub fn hash3(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    let mut h = seed
        ^ (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    h
}

/// Uniform float in `[0, 1]` for a lattice point.
pub fn rand3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    hash3(x, y, z, seed) as f32 / u32::MAX as f32
}

/// Smoothstep fade curve `3t² - 2t³`, which hides the lattice in value noise.
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Trilinearly interpolated value noise in `[0, 1]`.
pub fn value3([x, y, z]: [f32; 3], seed: u32) -> f32 {
    let (xf, yf, zf) = (x.floor(), y.floor(), z.floor());
    let (xi, yi, zi) = (xf as i32, yf as i32, zf as i32);
    let (u, v, w) = (smooth(x - xf), smooth(y - yf), smooth(z - zf));

    let c = |dx, dy, dz| rand3(xi + dx, yi + dy, zi + dz, seed);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;

    let x00 = lerp(c(0, 0, 0), c(1, 0, 0), u);
    let x10 = lerp(c(0, 1, 0), c(1, 1, 0), u);
    let x01 = lerp(c(0, 0, 1), c(1, 0, 1), u);
    let x11 = lerp(c(0, 1, 1), c(1, 1, 1), u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

/// Fractal sum of `octaves` layers of value noise, normalized to `[0, 1]`
/// and contrast-stretched (raw fBm clusters tightly around 0.5).
pub fn fbm(p: [f32; 3], seed: u32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..octaves {
        sum += amp * value3(p.map(|c| c * freq), seed.wrapping_add(o * 0x9E37));
        norm += amp;
        amp *= 0.5;
        freq *= 2.03;
    }
    ((sum / norm - 0.5) * 2.4 + 0.5).clamp(0.0, 1.0)
}

/// Tests for range, determinism and continuity of the noise.
#[cfg(test)]
mod tests {
    use super::*;

    /// The same input gives the same value, always within `0..=1`.
    #[test]
    fn noise_is_deterministic_and_in_range() {
        for i in 0..500 {
            let p = [i as f32 * 0.37, i as f32 * -0.21, i as f32 * 0.11];
            let a = fbm(p, 42, 5);
            assert_eq!(a, fbm(p, 42, 5));
            assert!((0.0..=1.0).contains(&a));
            assert!((0.0..=1.0).contains(&value3(p, 7)));
        }
    }

    /// Nearby points have nearby values.
    #[test]
    fn noise_is_continuous() {
        let a = value3([1.5, 2.25, 3.0], 1);
        let b = value3([1.501, 2.25, 3.0], 1);
        assert!((a - b).abs() < 0.01);
    }

    /// Different seeds give different noise.
    #[test]
    fn seeds_differ() {
        let p = [0.3, 0.6, 0.9];
        assert_ne!(value3(p, 1), value3(p, 2));
    }

    /// The contrast stretch spreads fBm values into both tails.
    #[test]
    fn fbm_spreads_across_range() {
        let samples: Vec<f32> = (0..2000)
            .map(|i| {
                fbm(
                    [i as f32 * 0.173, (i as f32 * 0.071).sin() * 9.0, 0.5],
                    3,
                    5,
                )
            })
            .collect();
        let low = samples.iter().filter(|&&v| v < 0.3).count();
        let high = samples.iter().filter(|&&v| v > 0.7).count();
        assert!(low > 50 && high > 50, "low {low}, high {high}");
    }

    /// The lattice hash depends on every coordinate and the seed.
    #[test]
    fn hash_depends_on_all_inputs() {
        let h = hash3(1, 2, 3, 4);
        assert_eq!(h, hash3(1, 2, 3, 4));
        assert_ne!(h, hash3(2, 2, 3, 4));
        assert_ne!(h, hash3(1, 3, 3, 4));
        assert_ne!(h, hash3(1, 2, 4, 4));
        assert_ne!(h, hash3(1, 2, 3, 5));
    }

    /// Value noise equals the lattice value at integer points.
    #[test]
    fn value_noise_hits_lattice_points() {
        assert_eq!(value3([3.0, -2.0, 5.0], 9), rand3(3, -2, 5, 9));
    }

    /// The fade curve is 0 at 0, 1 at 1 and symmetric around 0.5.
    #[test]
    fn smooth_is_a_symmetric_s_curve() {
        assert_eq!(smooth(0.0), 0.0);
        assert_eq!(smooth(1.0), 1.0);
        assert!((smooth(0.25) + smooth(0.75) - 1.0).abs() < 1e-6);
    }
}
