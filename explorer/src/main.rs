//! A 320x200 pixel galaxy explorer for the Elite universe.
//!
//! Every frame is drawn on the CPU into a small RGBA buffer, uploaded as a
//! texture and scaled up by the largest integer factor that fits the window.
//!
//! Keys: Left/Right (A/D) travel between systems, hold to zip; Up/Down (W/S)
//! hyperspace to the next/previous galaxy; Tab or C toggles the galactic
//! chart; Space pauses planet rotation; Esc quits.

mod app;
mod fb;
mod noise;
mod planet;
mod scene;

use game_toolkit::prelude::*;

use app::{App, Controls};
use fb::{H, W};

/// The game-toolkit adapter: owns the GPU texture and forwards input to [`App`].
struct Explorer {
    /// Texture the framebuffer is uploaded to every frame.
    texture: TextureId,
    /// Everything that is not GPU state.
    app: App,
}

/// Turns two key groups into -1, 0 or +1 using `f` (pressed or held); both cancel out.
fn axis(input: &Input, plus: [Key; 2], minus: [Key; 2], f: fn(&Input, Key) -> bool) -> i32 {
    i32::from(plus.iter().any(|&k| f(input, k))) - i32::from(minus.iter().any(|&k| f(input, k)))
}

impl Game for Explorer {
    /// Builds the app state and the texture its framebuffer is uploaded to.
    fn init(ctx: &mut Context) -> Result<Self> {
        let app = App::new();
        let texture =
            ctx.gfx
                .create_texture_rgba(W as u32, H as u32, &app.fb.pixels, Some("explorer"));
        Ok(Self { texture, app })
    }

    /// Maps keys to [`Controls`] and advances the app; Esc quits.
    fn update(&mut self, ctx: &mut Context, dt: f32) {
        let input = &ctx.input;
        if input.key_pressed(Key::Escape) {
            ctx.quit();
            return;
        }
        let right = [Key::ArrowRight, Key::KeyD];
        let left = [Key::ArrowLeft, Key::KeyA];
        let controls = Controls {
            step: axis(input, right, left, Input::key_pressed),
            held: axis(input, right, left, Input::key_held),
            galaxy: axis(
                input,
                [Key::ArrowUp, Key::KeyW],
                [Key::ArrowDown, Key::KeyS],
                Input::key_pressed,
            ),
            toggle_chart: input.key_pressed(Key::Tab) || input.key_pressed(Key::KeyC),
            toggle_pause: input.key_pressed(Key::Space),
        };
        self.app.update(controls, dt);
    }

    /// Draws the frame on the CPU, uploads it and scales it to the window by a whole number.
    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        self.app.draw();
        ctx.gfx
            .update_texture_rgba(self.texture, &self.app.fb.pixels);

        // Largest integer scale that fits, centred: crisp, square pixels.
        let (w, h) = ctx.gfx.size();
        let k = (w / W as u32).min(h / H as u32).max(1) as f32;
        let size = [W as f32 * k, H as f32 * k];
        let pos = [
            ((w as f32 - size[0]) / 2.0).floor(),
            ((h as f32 - size[1]) / 2.0).floor(),
        ];

        let mut p = frame.painter(&mut ctx.gfx);
        p.clear([0.0, 0.0, 0.0, 1.0]);
        p.sprite(self.texture, pos, size);
    }
}

/// Opens a 960x600 window (3x scale) and runs the explorer.
fn main() -> Result<()> {
    env_logger::init();
    run::<Explorer>(AppConfig {
        title: "Elite Galaxy Explorer".into(),
        width: W as u32 * 3,
        height: H as u32 * 3,
        ..Default::default()
    })
}
