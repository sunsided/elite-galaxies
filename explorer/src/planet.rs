//! Planet looks derived from a system's data and description, and a per-pixel
//! sphere renderer that turns them into dithered pixel art.

use elite::StarSystem;

use crate::fb::{Fb, Rgb, bayer, dither_levels, lerp, scale};
use crate::noise::{fbm, hash3, rand3, value3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The dominant terrain type, which picks palette, sea level, clouds and atmosphere.
pub enum Surface {
    /// Mostly water with green islands ("oceans").
    Ocean,
    /// Dense green continents ("rain forests", "tropical forests").
    Jungle,
    /// Patchwork fields of crops ("plantations").
    Plantation,
    /// Bare rock and snowy peaks ("mountains", "rock formations").
    Rocky,
    /// Dark rock with glowing lava seas ("volcanoes").
    Volcanic,
    /// Frozen seas and white land; never named by a description, picked from the stats.
    Ice,
    /// Sand and dry basins under a dusty sky.
    Dusty,
    /// A grey world covered by cities, with highways by day and lights by night.
    City,
    /// Grey airless rock; used for moons.
    Barren,
}

#[derive(Clone, Debug)]
/// Surface colours of one planet.
pub struct Palette {
    /// Colour of the deepest water (or lava).
    pub deep: Rgb,
    /// Colour of water near the coast.
    pub shallow: Rgb,
    /// Lowland to peaks.
    pub land: [Rgb; 4],
}

#[derive(Clone, Debug)]
/// A ring system around the planet's equator.
pub struct Rings {
    /// Inner and outer edge, in planet radii.
    pub inner: f32,
    /// Outer edge, in planet radii.
    pub outer: f32,
    /// Rotation of the ring ellipse on screen, in radians.
    pub tilt: f32,
    /// Base colour of the ring material.
    pub color: Rgb,
}

#[derive(Clone, Debug)]
/// A small moon on an inclined orbit, passing in front of and behind the planet.
pub struct Moon {
    /// Orbit radius and moon radius, in planet radii.
    pub distance: f32,
    /// Moon radius, in planet radii.
    pub size: f32,
    /// Radians per second.
    pub speed: f32,
    /// Angle on the orbit at time zero, in radians.
    pub phase: f32,
    /// Noise seed for the moon's craters.
    pub seed: u32,
}

#[derive(Clone, Debug)]
/// Everything that decides how one planet is drawn, derived from its system.
pub struct Look {
    /// Noise seed, mixed from the system's 48-bit seed.
    pub seed: u32,
    /// Terrain type.
    pub surface: Surface,
    /// Surface colours.
    pub palette: Palette,
    /// Height below which the surface is water (or lava, or dry basins).
    pub sea_level: f32,
    /// Absolute latitude (0 equator, 1 pole) above which ice caps start; > 1 means none.
    pub ice_caps: f32,
    /// Cloud coverage, 0..1.
    pub clouds: f32,
    /// Colour of the clouds (white, ash, dust or a sickly green).
    pub cloud_color: Rgb,
    /// Colour of the atmospheric haze around the limb, or `None` for airless bodies.
    pub atmosphere: Option<Rgb>,
    /// Density of city lights on the night side, 0..1.
    pub lights: f32,
    /// Lowlands drawn as fields in alternating crop colours.
    pub patchwork: bool,
    /// Water areas glow as lava.
    pub lava: bool,
    /// Flickering fires on land ("civil war").
    pub fires: bool,
    /// Glowing fault lines ("earthquakes").
    pub quakes: bool,
    /// Ring system, if any.
    pub rings: Option<Rings>,
    /// Moon, if any.
    pub moon: Option<Moon>,
    /// Axial tilt on screen, radians.
    pub tilt: f32,
    /// Spin, radians per second.
    pub spin: f32,
    /// Radius in pixels at normal zoom.
    pub size: f32,
    /// Colour of the system's star.
    pub star: Rgb,
    /// Whether the star throws flares ("solar activity").
    pub flares: bool,
}

/// Star colours from yellow and orange dwarfs to red, white and blue-white stars.
const STAR_COLORS: [Rgb; 6] = [
    [255, 240, 180],
    [255, 226, 140],
    [255, 172, 84],
    [255, 112, 72],
    [236, 240, 255],
    [172, 200, 255],
];

/// Adjectives that shift a planet's colours into alien territory.
const WEIRD: [&str; 5] = ["funny", "wierd", "unusual", "strange", "peculiar"];

impl Look {
    /// Reads the system's description and stats and decides what it looks like.
    pub fn for_system(s: &StarSystem) -> Self {
        let seed = (u32::from(s.seed.s0) << 16 | u32::from(s.seed.s1))
            ^ u32::from(s.seed.s2).rotate_left(11);
        let r = |k: i32| rand3(k, 17, 4, seed);

        let desc = s.description().to_lowercase();
        let has = |w: &str| desc.contains(w);
        let industrial = s.economy <= 3;

        let surface = if has("volcano") {
            Surface::Volcanic
        } else if has("ice berg") {
            Surface::Ice
        } else if has("ocean") {
            Surface::Ocean
        } else if has("forest") {
            Surface::Jungle
        } else if has("plantation") {
            Surface::Plantation
        } else if has("mountain") || has("rock formation") {
            Surface::Rocky
        } else if has("dust cloud") {
            Surface::Dusty
        } else if has("parking meter") {
            Surface::City
        } else {
            match s.economy {
                0 | 1 if s.tech_level >= 9 => Surface::City,
                // No description ever mentions "ice bergs" (the RNG never picks
                // that phrase), so frozen worlds come from the stats instead:
                // half the poor industrial worlds are icy mining outposts.
                2 if r(16) < 0.5 => Surface::Ice,
                0..=3 => Surface::Rocky,
                4 | 5 if r(1) < 0.5 => Surface::Jungle,
                4 | 5 => Surface::Ocean,
                6 => Surface::Plantation,
                _ => Surface::Dusty,
            }
        };

        let (mut palette, sea_level, clouds, cloud_color, atmosphere) = base_style(surface);

        // "pink" colours whatever it describes: "pink oceans" or the land.
        let pink = [232, 120, 176];
        if has("pink ocean") {
            palette.deep = lerp(palette.deep, pink, 0.5);
            palette.shallow = lerp(palette.shallow, pink, 0.65);
        } else if has("pink") {
            for c in &mut palette.land {
                *c = lerp(*c, pink, 0.55);
            }
        }
        if WEIRD.iter().any(|w| has(w)) {
            // Rotate the colour channels: familiar shapes, alien colours.
            for c in &mut palette.land {
                *c = [c[1], c[2], c[0]];
            }
        }

        let cloud_color = if has("disease") {
            lerp(cloud_color, [140, 214, 92], 0.6)
        } else {
            cloud_color
        };

        let ice_caps = match surface {
            Surface::Ocean | Surface::Jungle | Surface::Plantation | Surface::Rocky => {
                0.76 + r(3) * 0.14
            }
            Surface::Dusty => 0.93,
            _ => 2.0,
        };

        let lights = match surface {
            Surface::Barren => 0.0,
            Surface::City => 1.0,
            _ => (s.tech_level as f32 + 1.0) / 15.0 * if industrial { 1.0 } else { 0.45 },
        };

        let rings = (r(4) < 0.2).then(|| Rings {
            inner: 1.3 + r(5) * 0.2,
            outer: 1.75 + r(6) * 0.45,
            tilt: (r(7) - 0.5) * 0.6,
            color: lerp([200, 184, 150], [170, 176, 190], r(8)),
        });

        let moon = (r(9) < 0.45).then(|| Moon {
            distance: 1.7 + r(10) * 0.6,
            size: 0.14 + r(11) * 0.1,
            speed: 0.25 + r(12) * 0.3,
            phase: r(13) * std::f32::consts::TAU,
            seed: seed ^ 0x0004_D00E,
        });

        // Radius 2816..=6911 km maps to 30..=50 px.
        let mut size = 30.0 + (s.radius as f32 - 2816.0) / (6911.0 - 2816.0) * 20.0;
        if rings.is_some() {
            size *= 0.82;
        }

        Self {
            seed,
            surface,
            palette,
            sea_level,
            ice_caps,
            clouds: ((clouds + (r(2) - 0.5) * 0.2) * 0.7).clamp(0.0, 0.8),
            cloud_color,
            atmosphere,
            lights,
            patchwork: surface == Surface::Plantation,
            lava: surface == Surface::Volcanic,
            fires: has("civil war"),
            quakes: has("earthquake"),
            rings,
            moon,
            tilt: (r(14) - 0.5) * 0.7,
            spin: 0.12 + r(15) * 0.22,
            size,
            star: STAR_COLORS[(hash3(seed as i32, 1, 2, 3) % 6) as usize],
            flares: has("solar activity"),
        }
    }

    /// A grey cratered moon.
    pub fn moon(seed: u32) -> Self {
        let (palette, sea_level, ..) = base_style(Surface::Barren);
        Self {
            seed,
            surface: Surface::Barren,
            palette,
            sea_level,
            ice_caps: 2.0,
            clouds: 0.0,
            cloud_color: [0; 3],
            atmosphere: None,
            lights: 0.0,
            patchwork: false,
            lava: false,
            fires: false,
            quakes: false,
            rings: None,
            moon: None,
            tilt: 0.0,
            spin: 0.05,
            size: 0.0,
            star: [255; 3],
            flares: false,
        }
    }
}

/// Palette, sea level, cloud coverage, cloud colour and atmosphere per surface.
fn base_style(surface: Surface) -> (Palette, f32, f32, Rgb, Option<Rgb>) {
    let p = |deep, shallow, land| Palette {
        deep,
        shallow,
        land,
    };
    let white = [236, 238, 242];
    let sky = Some([110, 170, 255]);
    match surface {
        Surface::Ocean => (
            p(
                [12, 28, 88],
                [36, 92, 170],
                [
                    [46, 128, 64],
                    [96, 150, 70],
                    [140, 128, 92],
                    [220, 220, 215],
                ],
            ),
            0.6,
            0.5,
            white,
            sky,
        ),
        Surface::Jungle => (
            p(
                [16, 44, 100],
                [34, 100, 140],
                [[18, 84, 34], [30, 112, 42], [56, 136, 52], [86, 104, 58]],
            ),
            0.42,
            0.55,
            white,
            Some([120, 200, 220]),
        ),
        Surface::Plantation => (
            p(
                [24, 56, 118],
                [50, 110, 168],
                [
                    [110, 150, 58],
                    [178, 168, 72],
                    [132, 104, 58],
                    [208, 208, 190],
                ],
            ),
            0.4,
            0.35,
            white,
            sky,
        ),
        Surface::Rocky => (
            p(
                [34, 44, 74],
                [58, 78, 108],
                [
                    [104, 86, 68],
                    [136, 116, 92],
                    [166, 156, 146],
                    [236, 236, 236],
                ],
            ),
            0.3,
            0.25,
            white,
            Some([150, 170, 210]),
        ),
        Surface::Volcanic => (
            p(
                [255, 70, 0],
                [255, 170, 40],
                [[42, 32, 30], [66, 48, 42], [92, 74, 66], [124, 114, 108]],
            ),
            0.28,
            0.3,
            [92, 86, 80],
            Some([220, 90, 50]),
        ),
        Surface::Ice => (
            p(
                [120, 170, 210],
                [176, 214, 236],
                [
                    [214, 230, 242],
                    [236, 244, 252],
                    [178, 198, 218],
                    [255, 255, 255],
                ],
            ),
            0.45,
            0.3,
            white,
            Some([170, 220, 255]),
        ),
        Surface::Dusty => (
            p(
                [118, 78, 48],
                [150, 102, 62],
                [
                    [188, 138, 80],
                    [208, 168, 100],
                    [168, 118, 70],
                    [232, 204, 156],
                ],
            ),
            0.25,
            0.45,
            [206, 164, 112],
            Some([230, 160, 90]),
        ),
        Surface::City => (
            p(
                [26, 36, 58],
                [44, 56, 80],
                [
                    [84, 84, 90],
                    [104, 100, 96],
                    [126, 126, 132],
                    [156, 156, 162],
                ],
            ),
            0.35,
            0.3,
            [152, 150, 130],
            Some([180, 170, 140]),
        ),
        Surface::Barren => (
            p(
                [70, 70, 72],
                [90, 90, 92],
                [
                    [110, 108, 104],
                    [130, 128, 124],
                    [150, 148, 144],
                    [176, 174, 170],
                ],
            ),
            0.25,
            0.0,
            [0; 3],
            None,
        ),
    }
}

/// Dot product of two 3D vectors.
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Draws planet, rings and moon centred at `(cx, cy)` with radius `r`.
///
/// `light` points towards the sun, in screen space (x right, y down, z out).
pub fn draw(fb: &mut Fb, look: &Look, cx: f32, cy: f32, r: f32, t: f32, light: [f32; 3]) {
    if r < 1.0 {
        fb.set(cx as i32, cy as i32, look.palette.land[1]);
        return;
    }

    let moon_pos = look.moon.as_ref().map(|m| {
        let a = m.phase + t * m.speed;
        let (s, c) = a.sin_cos();
        (
            m,
            cx + c * m.distance * r,
            cy + s * m.distance * r * 0.3,
            s < 0.0, // behind the planet
        )
    });

    if let Some((m, mx, my, true)) = moon_pos {
        sphere(fb, &Look::moon(m.seed), mx, my, m.size * r, t, light);
    }
    if let Some(rings) = &look.rings {
        draw_rings(fb, look.seed, rings, cx, cy, r, false);
    }

    sphere(fb, look, cx, cy, r, t, light);

    if let Some(rings) = &look.rings {
        draw_rings(fb, look.seed, rings, cx, cy, r, true);
    }
    if let Some((m, mx, my, false)) = moon_pos {
        sphere(fb, &Look::moon(m.seed), mx, my, m.size * r, t, light);
    }
}

/// Draws one shaded sphere (planet or moon) with its atmospheric halo.
fn sphere(fb: &mut Fb, look: &Look, cx: f32, cy: f32, r: f32, t: f32, light: [f32; 3]) {
    let (st, ct) = look.tilt.sin_cos();
    let (sr, cr) = (t * look.spin).sin_cos();
    let halo = if look.atmosphere.is_some() {
        r * 1.12 + 1.0
    } else {
        r + 1.0
    };

    let x0 = ((cx - halo).floor() as i32).max(0);
    let x1 = ((cx + halo).ceil() as i32).min(crate::fb::W - 1);
    let y0 = ((cy - halo).floor() as i32).max(0);
    let y1 = ((cy + halo).ceil() as i32).min(crate::fb::H - 1);

    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = (x as f32 + 0.5 - cx) / r;
            let dy = (y as f32 + 0.5 - cy) / r;
            let d2 = dx * dx + dy * dy;

            if d2 > 1.0 {
                if let Some(atmo) = look.atmosphere {
                    let d = d2.sqrt();
                    let fade = 1.0 - (d - 1.0) * r / (r * 0.12 + 1.0);
                    let lit = ((dx * light[0] + dy * light[1]) / d * 0.8 + 0.35).clamp(0.0, 1.0);
                    if fade > 0.0 {
                        fb.set_dithered(x, y, scale(atmo, 0.4 + lit * 0.6), fade * lit * 0.7);
                    }
                }
                continue;
            }

            let nz = (1.0 - d2).sqrt();
            let lambert = dot([dx, dy, nz], light);
            // Undo the axial tilt, then spin about the planet's axis.
            let (bx, by) = (dx * ct + dy * st, -dx * st + dy * ct);
            let p = [bx * cr + nz * sr, by, -bx * sr + nz * cr];
            let c = surface_color(look, p, nz, lambert, t, x, y);
            fb.set(x, y, c);
        }
    }
}

/// Hash of the lattice cell of `p` at grid frequency `k`, for per-cell features.
fn cell(p: [f32; 3], k: f32, seed: u32) -> u32 {
    hash3(
        (p[0] * k).floor() as i32,
        (p[1] * k).floor() as i32,
        (p[2] * k).floor() as i32,
        seed,
    )
}

/// Colour of the surface point `p` on the unit sphere, shaded by `lambert`, at pixel `(x, y)`.
fn surface_color(look: &Look, p: [f32; 3], nz: f32, lambert: f32, t: f32, x: i32, y: i32) -> Rgb {
    let seed = look.seed;
    let pal = &look.palette;
    let h = fbm(p.map(|c| c * 1.8), seed, 5);
    let day = (lambert * 1.2 + 0.06).clamp(0.0, 1.0);
    let shade = dither_levels(0.1 + day * 0.9, 5.0, x, y);

    let water = h < look.sea_level;
    let mut col;
    let mut glow = None;

    if water {
        let depth = h / look.sea_level;
        col = lerp(
            pal.deep,
            pal.shallow,
            dither_levels(depth * depth, 2.0, x, y),
        );
        if look.lava {
            let pulse = 0.5 + 0.5 * (t * 2.0 + h * 40.0).sin();
            glow = Some(lerp(pal.deep, pal.shallow, depth * 0.7 + pulse * 0.3));
        }
    } else {
        let f = (h - look.sea_level) / (1.0 - look.sea_level);
        let band = ((f * 3.6 + bayer(x, y) * 0.6 - 0.3).floor().max(0.0) as usize).min(3);
        col = pal.land[band];
        if look.patchwork && band < 2 {
            col = match cell(p, 26.0, seed ^ 0x00F1_E1D5) % 4 {
                0 => pal.land[0],
                1 => pal.land[1],
                2 => scale(pal.land[1], 0.82),
                _ => lerp(pal.land[0], [96, 70, 40], 0.4),
            };
        }
        if look.surface == Surface::City && band < 3 {
            let gx = (p[0] * 28.0).rem_euclid(1.0) < 0.14;
            let gz = (p[2] * 28.0).rem_euclid(1.0) < 0.14;
            if gx || gz {
                col = scale(col, 1.35); // sunlit highways between city blocks
            }
        }
    }

    // Polar ice, with a ragged edge.
    let ice = p[1].abs() + (value3(p.map(|c| c * 6.0), seed ^ 0x1CE) - 0.5) * 0.18 > look.ice_caps;
    if ice {
        col = [232, 240, 250];
        glow = None;
    }

    let mut out = scale(col, shade);
    if let Some(g) = glow {
        out = g;
    }

    if look.quakes {
        let fault = (fbm(p.map(|c| c * 2.4), seed ^ 0xFA17, 4) - 0.5).abs() < 0.016;
        if fault {
            out = if day > 0.25 {
                scale(out, 0.45)
            } else {
                [255, 120, 40]
            };
        }
    }

    let cloud = fbm(
        [p[0] * 2.4 + t * 0.015, p[1] * 4.8, p[2] * 2.4],
        seed ^ 0xC10D,
        4,
    ) > 1.0 - look.clouds;

    if cloud {
        return scale(
            look.cloud_color,
            dither_levels(0.12 + day * 0.88, 5.0, x, y),
        );
    }

    if !water && !ice {
        if look.lights > 0.0 && day < 0.04 {
            let c = cell(p, 60.0, seed ^ 0x0001_1647);
            let density = if look.surface == Surface::City {
                0.3
            } else {
                0.14
            };
            if (c % 1000) as f32 / 1000.0 < look.lights * density {
                out = if c & 0x1000 == 0 {
                    [255, 214, 120]
                } else {
                    [200, 150, 80]
                };
            }
        }
        if look.fires {
            let c = cell(p, 30.0, seed ^ 0xF14E);
            if c % 100 < 3 {
                let flicker = hash3(c as i32, (t * 9.0) as i32, 0, seed);
                if !flicker.is_multiple_of(3) {
                    out = if flicker & 4 == 0 {
                        [255, 84, 20]
                    } else {
                        [255, 196, 64]
                    };
                }
            }
        }
    }

    // Limb haze: the atmosphere thickens towards the edge on the day side.
    if let Some(atmo) = look.atmosphere {
        let limb = 1.0 - nz;
        if limb > 0.72 && (limb - 0.72) * 3.0 * day > bayer(x, y) {
            out = lerp(out, atmo, 0.55);
        }
    }

    out
}

/// Draws the half of the rings in front of (`front`) or behind the planet.
fn draw_rings(fb: &mut Fb, seed: u32, rings: &Rings, cx: f32, cy: f32, r: f32, front: bool) {
    /// Vertical squash of the ring ellipse: how flat we see the ring plane.
    const SQUASH: f32 = 0.28;
    let outer = rings.outer * r;
    let (s, c) = rings.tilt.sin_cos();
    let x0 = ((cx - outer).floor() as i32).max(0);
    let x1 = ((cx + outer).ceil() as i32).min(crate::fb::W - 1);
    let reach = outer * (SQUASH + s.abs()) + 1.0;
    let y0 = ((cy - reach).floor() as i32).max(0);
    let y1 = ((cy + reach).ceil() as i32).min(crate::fb::H - 1);

    for y in y0..=y1 {
        for x in x0..=x1 {
            let (ox, oy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let u = ox * c + oy * s;
            let v = -ox * s + oy * c;
            if (v > 0.0) != front {
                continue;
            }
            let rr = (u * u + (v / SQUASH).powi(2)).sqrt() / r;
            if rr < rings.inner || rr > rings.outer {
                continue;
            }
            let band = value3([rr * 16.0, 0.5, 0.5], seed ^ 0x0002_1465);
            if band < 0.3 {
                continue; // a gap between ringlets
            }
            let k = dither_levels(0.45 + band * 0.5, 4.0, x, y);
            fb.set(x, y, scale(rings.color, k));
        }
    }
}

/// Tests for planet looks and the sphere renderer.
#[cfg(test)]
mod tests {
    use super::*;

    /// The look of the system called `name` in galaxy `galaxy` (0-based).
    fn look(name: &str, galaxy: usize) -> Look {
        let g = elite::galaxy(galaxy);
        let s = g.iter().find(|s| s.name == name).unwrap();
        Look::for_system(s)
    }

    /// Lave's rain forests make a jungle; Diso's plantations a patchwork with flares.
    #[test]
    fn descriptions_drive_the_surface() {
        // "vast rain forests and the Lavian tree grub"
        assert_eq!(look("Lave", 0).surface, Surface::Jungle);
        // "ancient Ma corn plantations ... frequent solar activity"
        let diso = look("Diso", 0);
        assert_eq!(diso.surface, Surface::Plantation);
        assert!(diso.patchwork && diso.flares);
    }

    /// Every system's look has size, clouds and lights within range.
    #[test]
    fn every_system_gets_a_sane_look() {
        for g in 0..8 {
            for s in elite::galaxy(g) {
                let l = Look::for_system(&s);
                assert!((24.0..=50.0).contains(&l.size), "{}: {}", s.name, l.size);
                assert!((0.0..=0.8).contains(&l.clouds));
                assert!((0.0..=1.0).contains(&l.lights));
            }
        }
    }

    /// Every surface kind except moon rock occurs somewhere.
    #[test]
    fn every_surface_kind_occurs_in_the_eight_galaxies() {
        let mut seen = std::collections::HashSet::new();
        for g in 0..8 {
            for s in elite::galaxy(g) {
                seen.insert(format!("{:?}", Look::for_system(&s).surface));
            }
        }
        assert_eq!(seen.len(), 8, "{seen:?}"); // all but Barren (moons only)
    }

    /// Drawing off-screen, tiny or huge planets does not panic.
    #[test]
    fn drawing_fully_offscreen_or_tiny_does_not_panic() {
        let mut fb = Fb::new();
        let l = look("Lave", 0);
        for (x, y, r) in [
            (-500.0, -500.0, 40.0),
            (900.0, 50.0, 40.0),
            (10.0, 10.0, 0.2),
            (160.0, 100.0, 300.0),
        ] {
            draw(&mut fb, &l, x, y, r, 1.0, [-0.6, -0.45, 0.66]);
        }
    }

    /// The side facing the sun is brighter than the far side.
    #[test]
    fn planet_is_lit_on_the_sun_side() {
        let mut fb = Fb::new();
        let l = look("Lave", 0);
        draw(&mut fb, &l, 160.0, 100.0, 40.0, 0.0, [-0.6, -0.45, 0.66]);
        let lum = |c: Rgb| c.iter().map(|&v| v as u32).sum::<u32>();
        let lit: u32 = (0..10).map(|i| lum(fb.get(140 + i, 85))).sum();
        let dark: u32 = (0..10).map(|i| lum(fb.get(180 + i, 115))).sum();
        assert!(lit > dark, "lit {lit} vs dark {dark}");
    }

    /// "pink oceans" tints the water, not the land.
    #[test]
    fn pink_oceans_tint_the_water() {
        let azaqu = look("Azaqu", 0);
        let plain = base_style(azaqu.surface).0;
        assert_ne!(azaqu.palette.shallow, plain.shallow);
        assert_eq!(azaqu.palette.land, plain.land);
    }

    /// Weird adjectives rotate the land colours' channels.
    #[test]
    fn weird_worlds_rotate_their_colours() {
        let raoror = look("Raoror", 0); // "wierd volcanoes"
        let plain = base_style(Surface::Volcanic).0.land[0];
        assert_eq!(raoror.palette.land[0], [plain[1], plain[2], plain[0]]);
    }

    /// Volcanoes give lava, civil war fires, solar activity flares.
    #[test]
    fn description_flags() {
        assert!(look("Raoror", 0).lava);
        assert!(look("Riedquat", 0).fires);
        assert!(look("Tibedied", 0).flares);
        assert!(!look("Lave", 0).fires);
    }

    /// Moons are airless grey rock without extras.
    #[test]
    fn moon_look_is_barren() {
        let m = Look::moon(7);
        assert_eq!(m.surface, Surface::Barren);
        assert!(m.atmosphere.is_none() && m.rings.is_none() && m.moon.is_none());
        assert_eq!(m.clouds, 0.0);
    }

    /// Ringed planets are drawn smaller so the rings fit on screen.
    #[test]
    fn ringed_planets_shrink() {
        for s in elite::galaxy(0) {
            let l = Look::for_system(&s);
            if l.rings.is_some() {
                assert!(l.size <= 50.0 * 0.82 + 1e-3, "{}: {}", s.name, l.size);
            }
        }
    }

    /// The vector helpers do what they say.
    #[test]
    fn dot_and_cell() {
        assert_eq!(dot([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), 32.0);
        assert_eq!(
            cell([0.10, 0.2, 0.3], 10.0, 1),
            cell([0.15, 0.25, 0.35], 10.0, 1)
        );
        assert_ne!(
            cell([0.10, 0.2, 0.3], 10.0, 1),
            cell([0.25, 0.2, 0.3], 10.0, 1)
        );
    }

    /// Only the front half of the rings overdraws the planet.
    #[test]
    fn rings_split_into_front_and_back() {
        let rings = Rings {
            inner: 1.3,
            outer: 2.0,
            tilt: 0.0,
            color: [200, 200, 200],
        };
        let count = |front| {
            let mut fb = Fb::new();
            draw_rings(&mut fb, 1, &rings, 160.0, 100.0, 40.0, front);
            let lit = |x: i32, y: i32| fb.get(x, y) != [0; 3];
            (
                (0..crate::fb::W).filter(|&x| lit(x, 107)).count(),
                (0..crate::fb::W).filter(|&x| lit(x, 93)).count(),
            )
        };
        let (front_below, front_above) = count(true);
        let (back_below, back_above) = count(false);
        assert!(front_below > 0 && front_above == 0);
        assert!(back_above > 0 && back_below == 0);
    }
}
