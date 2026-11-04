//! Explorer state, navigation and frame composition. Free of GPU types, so
//! frames can be rendered and inspected headlessly.

use elite::StarSystem;

use crate::fb::{Fb, H, W, wrap};
use crate::planet::{self, Look};
use crate::scene;

/// Seconds for a normal hop, a hop while zipping, and a galaxy jump.
const TRAVEL: f32 = 0.55;
/// Seconds per hop while zipping through systems (queued or held key).
const ZIP: f32 = 0.2;
/// Seconds for a full galaxy jump; the galaxy switches at the halfway flash.
pub const HYPER: f32 = 1.6;
/// Chart zoom progress per second (a full zoom takes 0.45 s).
const CHART_SPEED: f32 = 1.0 / 0.45;

/// Screen X of the planet centre in the system view.
const PLANET_X: f32 = 160.0;
/// Screen Y of the planet centre in the system view.
const PLANET_Y: f32 = 64.0;
/// The sun's centre X, centre Y and radius: a quarter disc in the top-left corner.
const SUN: (f32, f32, f32) = (-8.0, -6.0, 30.0);
/// Direction towards the sun from the planet (screen space, z out of the screen).
const LIGHT: [f32; 3] = [-0.86, -0.38, 0.34];

/// Top edge of the text panel at the bottom of the screen.
const PANEL_Y: i32 = 128;

/// What the explorer is currently animating.
pub enum Motion {
    /// Parked at the current system; the description is being typed or shown.
    Idle,
    /// Moving from one slot to another; `dir` is the side the new system comes from.
    Travel {
        /// Slot the trip started from.
        from: usize,
        /// Slot the trip ends at.
        to: usize,
        /// +1 when moving right (new system enters from the right), -1 when moving left.
        dir: f32,
        /// Seconds elapsed in this hop.
        t: f32,
        /// Total duration of this hop in seconds.
        dur: f32,
    },
    /// Jumping to another galaxy with streaking stars and a white flash.
    Hyper {
        /// Seconds elapsed in this jump.
        t: f32,
        /// Galaxy number (0-based) the jump ends in.
        to_galaxy: usize,
        /// Whether the galaxy has already been swapped at the halfway point.
        switched: bool,
    },
}

/// One loaded galaxy with everything precomputed that drawing needs per frame.
pub struct Galaxy {
    /// Galaxy number, 0-based.
    pub number: usize,
    /// All 256 systems, in the game's index order.
    pub systems: Vec<StarSystem>,
    /// Planet look per system, indexed like `systems`.
    looks: Vec<Look>,
    /// Description text per system, indexed like `systems`.
    descriptions: Vec<String>,
    /// System indices sorted left to right across the chart.
    pub order: Vec<usize>,
}

impl Galaxy {
    /// Generates galaxy `number` (0-based) and precomputes looks, descriptions and chart order.
    pub fn load(number: usize) -> Self {
        let systems = elite::galaxy(number);
        let looks = systems.iter().map(Look::for_system).collect();
        let descriptions = systems.iter().map(StarSystem::description).collect();
        let mut order: Vec<usize> = (0..systems.len()).collect();
        order.sort_by_key(|&i| (systems[i].x, systems[i].y));
        Self {
            number,
            systems,
            looks,
            descriptions,
            order,
        }
    }

    /// Maps a chart-order slot to the system's index in `systems`.
    pub fn at(&self, slot: usize) -> usize {
        self.order[slot]
    }

    /// The system at chart-order slot `slot`.
    pub fn system(&self, slot: usize) -> &StarSystem {
        &self.systems[self.at(slot)]
    }

    /// The slot of the system closest to chart position `(x, y)`.
    fn nearest_slot(&self, x: f32, y: f32) -> usize {
        let d = |slot: usize| {
            let s = self.system(slot);
            (s.x as f32 - x).powi(2) + (s.y as f32 - y).powi(2)
        };
        (0..self.order.len())
            .min_by(|&a, &b| d(a).total_cmp(&d(b)))
            .unwrap_or(0)
    }
}

/// Keyboard intent for one frame, decoupled from the input backend.
#[derive(Default, Clone, Copy)]
pub struct Controls {
    /// -1 / +1 when left / right was pressed this frame.
    pub step: i32,
    /// -1 / +1 while left / right is held.
    pub held: i32,
    /// -1 / +1 when down / up was pressed this frame.
    pub galaxy: i32,
    /// Tab or C was pressed: zoom to or from the galactic chart.
    pub toggle_chart: bool,
    /// Space was pressed: stop or resume planet rotation.
    pub toggle_pause: bool,
}

/// The explorer: navigation state, animation clocks and the framebuffer it draws into.
pub struct App {
    /// The 320x200 frame the explorer draws into.
    pub fb: Fb,
    /// Background stars for the parallax starfield.
    stars: Vec<scene::Star>,
    /// The galaxy being explored.
    pub galaxy: Galaxy,
    /// Chart-order slot of the current system (the departure slot while travelling).
    pub slot: usize,
    /// The running animation, if any.
    pub motion: Motion,
    /// Extra hops requested while one is still running.
    queued: i32,
    /// Wall-clock seconds since start; drives twinkling, flicker and the typewriter.
    t: f32,
    /// Rotation clock for planets and moons; stands still while paused.
    spin_t: f32,
    /// Whether planet rotation is paused.
    paused: bool,
    /// When the current system was reached; drives the typewriter text.
    arrived: f32,
    /// Chart zoom progress: 0 shows the system, 1 the galactic chart.
    chart: f32,
    /// The chart zoom progress `chart` is animating towards (0 or 1).
    chart_target: f32,
}

/// Smoothstep easing of `x`, clamped to `0..=1`.
fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Linear interpolation between two 2D points.
fn lerp2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

impl App {
    /// Starts in galaxy 1 at Lave, idle, with the chart closed.
    pub fn new() -> Self {
        let galaxy = Galaxy::load(0);
        // Start at Lave, of course.
        let slot = galaxy
            .order
            .iter()
            .position(|&i| galaxy.systems[i].name == "Lave")
            .unwrap_or(0);
        Self {
            fb: Fb::new(),
            stars: scene::starfield(170),
            galaxy,
            slot,
            motion: Motion::Idle,
            queued: 0,
            t: 0.0,
            spin_t: 0.0,
            paused: false,
            arrived: 0.0,
            chart: 0.0,
            chart_target: 0.0,
        }
    }

    /// Applies one frame of input and advances the simulation by `dt` seconds.
    pub fn update(&mut self, c: Controls, dt: f32) {
        if c.toggle_pause {
            self.paused = !self.paused;
        }
        if c.toggle_chart {
            self.chart_target = 1.0 - self.chart_target;
        }
        if c.step != 0 {
            self.step(c.step);
        }
        if c.galaxy != 0 {
            self.start_hyper(c.galaxy);
        }

        self.t += dt;
        if !self.paused {
            self.spin_t += dt;
        }
        let towards = self.chart_target - self.chart;
        self.chart += towards.signum() * (dt * CHART_SPEED).min(towards.abs());
        self.advance(c.held, dt);
    }

    /// Galactic chart coordinates of the system at `slot`.
    fn pos(&self, slot: usize) -> [f32; 2] {
        let s = self.galaxy.system(slot);
        [s.x as f32, s.y as f32]
    }

    /// Requests a hop by `dir` slots: starts one now, or queues it behind a running hop.
    fn step(&mut self, dir: i32) {
        match self.motion {
            Motion::Idle => self.start_travel(dir, TRAVEL),
            Motion::Travel { .. } => self.queued = (self.queued + dir).clamp(-8, 8),
            Motion::Hyper { .. } => {}
        }
    }

    /// Starts a hop of `dir` slots from the current slot, wrapping around the galaxy.
    fn start_travel(&mut self, dir: i32, dur: f32) {
        let n = self.galaxy.order.len() as i32;
        let to = (self.slot as i32 + dir).rem_euclid(n) as usize;
        self.motion = Motion::Travel {
            from: self.slot,
            to,
            dir: dir.signum() as f32,
            t: 0.0,
            dur,
        };
    }

    /// Starts a jump to the next (`dir` = 1) or previous (-1) galaxy, unless already moving.
    fn start_hyper(&mut self, dir: i32) {
        if matches!(self.motion, Motion::Idle) {
            self.queued = 0;
            let to = (self.galaxy.number as i32 + dir).rem_euclid(elite::GALAXIES as i32);
            self.motion = Motion::Hyper {
                t: 0.0,
                to_galaxy: to as usize,
                switched: false,
            };
        }
    }

    /// Advances the running animation by `dt` and chains the next hop while `held` or queued.
    fn advance(&mut self, held: i32, dt: f32) {
        match &mut self.motion {
            Motion::Idle => {}
            Motion::Travel { to, t, dur, .. } => {
                *t += dt;
                if *t >= *dur {
                    self.slot = *to;
                    self.arrived = self.t;
                    self.motion = Motion::Idle;
                    // Keep zipping while hops are queued or a key is held.
                    if self.queued != 0 {
                        let d = self.queued.signum();
                        self.queued -= d;
                        self.start_travel(d, ZIP);
                    } else if held != 0 {
                        self.start_travel(held, ZIP);
                    }
                }
            }
            Motion::Hyper {
                t,
                to_galaxy,
                switched,
            } => {
                *t += dt;
                if !*switched && *t >= HYPER / 2.0 {
                    *switched = true;
                    let s = self.galaxy.system(self.slot);
                    let (x, y) = (s.x as f32, s.y as f32);
                    self.galaxy = Galaxy::load(*to_galaxy);
                    // Arrive near the same chart position, like the real drive.
                    self.slot = self.galaxy.nearest_slot(x, y);
                }
                if *t >= HYPER {
                    self.motion = Motion::Idle;
                    self.arrived = self.t;
                }
            }
        }
    }

    /// Renders the current state into the framebuffer.
    pub fn draw(&mut self) {
        let chart = ease(self.chart);
        self.fb.clear([4, 4, 14]);

        // Camera position in galaxy coordinates, for the parallax starfield.
        let (cam, warp) = match self.motion {
            Motion::Travel {
                from, to, t, dur, ..
            } => (lerp2(self.pos(from), self.pos(to), ease(t / dur)), 0.0),
            Motion::Hyper { t, .. } => (self.pos(self.slot), 1.0 - (t / HYPER * 2.0 - 1.0).abs()),
            Motion::Idle => (self.pos(self.slot), 0.0),
        };
        scene::draw_starfield(&mut self.fb, &self.stars, cam, self.t, warp);
        if chart > 0.0 {
            self.fb.shade_rect(0, 0, W, H, 1.0 - chart * 0.7);
        }

        let chart_of = |g: &Galaxy, slot: usize| scene::chart_pos(g.system(slot));

        let (panel_slot, details, cursor) = match self.motion {
            Motion::Idle => {
                let at = chart_of(&self.galaxy, self.slot);
                self.draw_system(self.slot, 0.0, 1.0, at, chart);
                (self.slot, true, at)
            }
            Motion::Travel {
                from,
                to,
                dir,
                t,
                dur,
            } => {
                let e = ease(t / dur);
                let cursor = lerp2(chart_of(&self.galaxy, from), chart_of(&self.galaxy, to), e);
                let span = W as f32 + 40.0;
                self.draw_system(from, -dir * e * span, 1.0 - 0.45 * e, cursor, chart);
                self.draw_system(to, dir * (1.0 - e) * span, 0.55 + 0.45 * e, cursor, chart);
                (to, false, cursor)
            }
            Motion::Hyper { t, .. } => {
                let e = t / HYPER;
                // Shrink into the jump, grow out of it.
                let zoom = (e * 2.0 - 1.0).abs();
                let at = chart_of(&self.galaxy, self.slot);
                self.draw_system(self.slot, 0.0, zoom, at, chart);
                let flash = 1.0 - ((e - 0.5).abs() / 0.08).min(1.0);
                if flash > 0.0 {
                    for y in 0..H {
                        for x in 0..W {
                            self.fb.set_dithered(x, y, [255, 255, 255], flash);
                        }
                    }
                }
                if (self.t * 4.0).fract() < 0.6 {
                    self.fb
                        .text_shadow(W / 2 - 40, PANEL_Y - 16, "HYPERSPACE", [255, 120, 255]);
                }
                (self.slot, false, at)
            }
        };

        scene::draw_chart(&mut self.fb, &self.galaxy.systems, cursor, chart, self.t);

        if chart < 0.5 {
            self.draw_panel(panel_slot, details);
            self.draw_header(false);
        } else {
            let label = self.galaxy.system(panel_slot).name.to_uppercase();
            let x = (cursor[0] as i32 + 8).min(W - 8 * label.len() as i32 - 2);
            self.fb
                .text_shadow(x, cursor[1] as i32 - 12, &label, [255, 255, 80]);
            self.fb
                .text_shadow(6, H - 12, "<> SYSTEM   ^v GALAXY   TAB VIEW", [0, 200, 230]);
            self.draw_header(true);
        }
    }

    /// Draws one system's sun and planet, shifted by `ox` and scaled by `zoom`.
    fn draw_system(&mut self, slot: usize, ox: f32, zoom: f32, toward: [f32; 2], chart: f32) {
        let look = &self.galaxy.looks[self.galaxy.at(slot)];
        let sun_off = -chart * 90.0;
        if chart < 1.0 {
            scene::draw_sun(
                &mut self.fb,
                SUN.0 + ox + sun_off,
                SUN.1 + sun_off * 0.5,
                SUN.2,
                look.star,
                self.t,
                look.flares,
            );
        }
        // Zooming out to the chart pulls the planet into its star on the chart.
        let center = lerp2([PLANET_X + ox, PLANET_Y], toward, chart);
        let r = look.size * zoom * (1.0 - chart);
        if r > 0.0 {
            planet::draw(
                &mut self.fb,
                look,
                center[0],
                center[1],
                r,
                self.spin_t,
                LIGHT,
            );
        }
    }

    /// Draws the bottom text panel for `slot`; without `show_details` only the name line.
    fn draw_panel(&mut self, slot: usize, show_details: bool) {
        let idx = self.galaxy.at(slot);
        let s = &self.galaxy.systems[idx];
        let fb = &mut self.fb;
        fb.shade_rect(0, PANEL_Y, W, H - PANEL_Y, 0.3);
        for x in 0..W {
            fb.set(x, PANEL_Y, [0, 90, 110]);
        }

        let y = PANEL_Y + 4;
        fb.text_shadow(6, y, &s.name.to_uppercase(), [255, 255, 80]);
        let stats = format!(
            "TL{} {}.{}bn",
            s.displayed_tech_level(),
            s.population / 10,
            s.population % 10
        );
        fb.text_shadow(W - 6 - 8 * stats.len() as i32, y, &stats, [255, 255, 80]);

        if !show_details {
            return;
        }
        let line = format!("{}, {}", s.economy_name(), s.government_name());
        fb.text_shadow(6, y + 11, &line, [80, 220, 255]);
        fb.text_shadow(6, y + 21, &s.species, [150, 240, 150]);

        // Typewriter: reveal the description a few characters per frame.
        let visible = ((self.t - self.arrived) * 70.0).max(0.0) as usize;
        let mut shown = 0;
        for (row, text) in wrap(&self.galaxy.descriptions[idx], 38).iter().enumerate() {
            let take = visible.saturating_sub(shown).min(text.len());
            shown += text.len() + 1;
            fb.text_shadow(6, y + 34 + row as i32 * 9, &text[..take], [235, 235, 235]);
        }
    }

    /// Draws the top-right label: galaxy and system number, or the chart title.
    fn draw_header(&mut self, chart: bool) {
        let label = if chart {
            format!("GALACTIC CHART {}", self.galaxy.number + 1)
        } else {
            format!(
                "GALAXY {}  #{:03}",
                self.galaxy.number + 1,
                self.galaxy.at(self.slot)
            )
        };
        self.fb
            .text_shadow(W - 4 - 8 * label.len() as i32, 3, &label, [0, 200, 230]);
    }
}

/// Tests for navigation, animation chaining and frame drawing.
#[cfg(test)]
mod tests {
    use super::*;

    /// One frame at 60 Hz.
    const FRAME: f32 = 1.0 / 60.0;

    /// Applies `c` for one frame, then idles until `seconds` have passed.
    fn run(app: &mut App, c: Controls, seconds: f32) {
        app.update(c, FRAME);
        let mut t = FRAME;
        while t < seconds {
            app.update(Controls::default(), FRAME);
            t += FRAME;
        }
    }

    /// Controls for one press of the right arrow.
    fn right() -> Controls {
        Controls {
            step: 1,
            ..Default::default()
        }
    }

    /// The explorer starts at Lave.
    #[test]
    fn starts_at_lave() {
        let app = App::new();
        assert_eq!(app.galaxy.system(app.slot).name, "Lave");
    }

    /// Chart order runs left to right across the galaxy.
    #[test]
    fn order_runs_left_to_right() {
        let app = App::new();
        let xs: Vec<u8> = (0..256).map(|s| app.galaxy.system(s).x).collect();
        assert!(xs.windows(2).all(|w| w[0] <= w[1]));
    }

    /// One press travels to the next slot and comes to rest.
    #[test]
    fn a_hop_arrives_at_the_next_slot() {
        let mut app = App::new();
        let start = app.slot;
        run(&mut app, right(), TRAVEL + 0.1);
        assert!(matches!(app.motion, Motion::Idle));
        assert_eq!(app.slot, start + 1);
    }

    /// Presses during a hop are queued and played out afterwards.
    #[test]
    fn presses_during_travel_are_queued() {
        let mut app = App::new();
        let start = app.slot;
        app.update(right(), FRAME);
        app.update(right(), FRAME);
        app.update(right(), FRAME);
        run(&mut app, Controls::default(), TRAVEL + 2.0 * ZIP + 0.2);
        assert_eq!(app.slot, start + 3);
    }

    /// Holding the key keeps hopping at zip speed.
    #[test]
    fn holding_zips_through_systems() {
        let mut app = App::new();
        let start = app.slot;
        app.update(right(), FRAME);
        let held = Controls {
            held: 1,
            ..Default::default()
        };
        let mut t = 0.0;
        while t < TRAVEL + 5.0 * ZIP {
            app.update(held, FRAME);
            t += FRAME;
        }
        assert!(app.slot >= start + 5, "only reached {}", app.slot - start);
    }

    /// Going left from the first slot arrives at the last one.
    #[test]
    fn travel_wraps_around_the_galaxy() {
        let mut app = App::new();
        app.slot = 0;
        run(
            &mut app,
            Controls {
                step: -1,
                ..Default::default()
            },
            TRAVEL + 0.1,
        );
        assert_eq!(app.slot, 255);
    }

    /// A jump lands in the next galaxy near the same chart position, and wraps from 1 down to 8.
    #[test]
    fn hyperspace_lands_in_next_galaxy_near_same_spot() {
        let mut app = App::new();
        let (x, y) = {
            let s = app.galaxy.system(app.slot);
            (s.x as i32, s.y as i32)
        };
        run(
            &mut app,
            Controls {
                galaxy: 1,
                ..Default::default()
            },
            HYPER + 0.1,
        );
        assert_eq!(app.galaxy.number, 1);
        let s = app.galaxy.system(app.slot);
        assert!((s.x as i32 - x).abs() < 40 && (s.y as i32 - y).abs() < 40);

        // Down from galaxy 1 wraps to galaxy 8.
        let mut app = App::new();
        run(
            &mut app,
            Controls {
                galaxy: -1,
                ..Default::default()
            },
            HYPER + 0.1,
        );
        assert_eq!(app.galaxy.number, 7);
    }

    /// Every animation phase draws without panicking.
    #[test]
    fn every_phase_draws_without_panicking() {
        let mut app = App::new();
        app.draw();
        app.update(right(), FRAME);
        for _ in 0..20 {
            app.update(Controls::default(), FRAME);
            app.draw();
        }
        app.update(
            Controls {
                toggle_chart: true,
                ..Default::default()
            },
            FRAME,
        );
        for _ in 0..40 {
            app.update(Controls::default(), FRAME);
            app.draw();
        }
        app.update(
            Controls {
                galaxy: 1,
                ..Default::default()
            },
            FRAME,
        );
        for _ in 0..100 {
            app.update(Controls::default(), FRAME);
            app.draw();
        }
    }

    /// Toggling the chart animates to it and back again.
    #[test]
    fn chart_toggle_animates_both_ways() {
        let mut app = App::new();
        let toggle = Controls {
            toggle_chart: true,
            ..Default::default()
        };
        run(&mut app, toggle, 0.1);
        assert!(app.chart > 0.0 && app.chart < 1.0);
        run(&mut app, Controls::default(), 1.0);
        assert_eq!(app.chart, 1.0);
        run(&mut app, toggle, 1.0);
        assert_eq!(app.chart, 0.0);
    }

    /// Pausing freezes the rotation clock but not wall time.
    #[test]
    fn pause_freezes_rotation_only() {
        let mut app = App::new();
        let pause = Controls {
            toggle_pause: true,
            ..Default::default()
        };
        run(&mut app, pause, 1.0);
        assert!(app.paused);
        assert_eq!(app.spin_t, 0.0);
        assert!(app.t > 0.9);
    }

    /// Left/right presses during a galaxy jump are ignored.
    #[test]
    fn hops_are_ignored_during_hyperspace() {
        let mut app = App::new();
        let start = app.slot;
        app.update(
            Controls {
                galaxy: 1,
                ..Default::default()
            },
            FRAME,
        );
        app.update(right(), FRAME);
        assert_eq!(app.queued, 0);
        assert!(matches!(app.motion, Motion::Hyper { .. }));
        assert_eq!(app.slot, start);
    }

    /// The nearest slot to a system's own coordinates is that system.
    #[test]
    fn nearest_slot_finds_exact_match() {
        let g = Galaxy::load(0);
        for slot in [0, 17, 128, 255] {
            let s = g.system(slot);
            let found = g.system(g.nearest_slot(s.x as f32, s.y as f32));
            assert_eq!((found.x, found.y), (s.x, s.y));
        }
    }

    /// Easing is clamped smoothstep, and `lerp2` interpolates both axes.
    #[test]
    fn easing_helpers() {
        assert_eq!(ease(-1.0), 0.0);
        assert_eq!(ease(0.5), 0.5);
        assert_eq!(ease(2.0), 1.0);
        assert_eq!(lerp2([0.0, 10.0], [10.0, 30.0], 0.5), [5.0, 20.0]);
    }

    /// The description types itself out after arriving.
    #[test]
    fn description_types_out() {
        let lit_rows = |app: &App| {
            (PANEL_Y + 38..H)
                .filter(|&y| (0..W).any(|x| app.fb.get(x, y) == [235, 235, 235]))
                .count()
        };
        let mut app = App::new();
        app.draw();
        let early = lit_rows(&app);
        run(&mut app, Controls::default(), 3.0);
        app.draw();
        assert!(lit_rows(&app) > early);
    }

    /// Writes PNG snapshots of several states to `$SNAPSHOT_DIR` for eyeballing.
    /// Run with `SNAPSHOT_DIR=/some/dir cargo test -p elite-explorer -- --ignored`.
    #[test]
    #[ignore = "writes image files; run on demand"]
    fn snapshots() {
        let dir = std::path::PathBuf::from(std::env::var("SNAPSHOT_DIR").expect("SNAPSHOT_DIR"));
        let save = |app: &mut App, name: &str| {
            app.draw();
            let file = std::fs::File::create(dir.join(format!("{name}.png"))).unwrap();
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W as u32, H as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()
                .unwrap()
                .write_image_data(&app.fb.pixels)
                .unwrap();
        };

        let mut app = App::new();
        run(&mut app, Controls::default(), 3.0);
        save(&mut app, "01_lave");

        app.update(right(), FRAME);
        run(&mut app, Controls::default(), TRAVEL * 0.45);
        save(&mut app, "02_travel");

        run(&mut app, Controls::default(), 3.0);
        save(&mut app, "03_next");

        for name in ["Diso", "Leesti", "Zaonce", "Riedquat", "Tionisla", "Orerve"] {
            app.slot = app
                .galaxy
                .order
                .iter()
                .position(|&i| app.galaxy.systems[i].name == name)
                .unwrap();
            app.arrived = app.t - 5.0;
            save(&mut app, &format!("10_{}", name.to_lowercase()));
        }

        app.update(
            Controls {
                toggle_chart: true,
                ..Default::default()
            },
            FRAME,
        );
        run(&mut app, Controls::default(), 0.2);
        save(&mut app, "20_chart_zoom");
        run(&mut app, Controls::default(), 1.0);
        save(&mut app, "21_chart");

        app.update(
            Controls {
                toggle_chart: true,
                ..Default::default()
            },
            FRAME,
        );
        run(&mut app, Controls::default(), 1.0);
        app.update(
            Controls {
                galaxy: 1,
                ..Default::default()
            },
            FRAME,
        );
        run(&mut app, Controls::default(), HYPER * 0.3);
        save(&mut app, "30_hyper");

        // A sample of every surface kind from across the galaxies.
        let mut seen = std::collections::HashSet::new();
        for g in 0..8 {
            app.galaxy = Galaxy::load(g);
            for slot in 0..256 {
                let kind = format!("{:?}", app.galaxy.looks[app.galaxy.at(slot)].surface);
                if seen.insert(kind.clone()) {
                    app.slot = slot;
                    app.motion = Motion::Idle;
                    app.arrived = app.t - 5.0;
                    save(&mut app, &format!("40_{}", kind.to_lowercase()));
                }
            }
        }
    }
}
