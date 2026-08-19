//! Insiculous Snake — game crate.
//!
//! The library owns the whole game (`SnakeGame` + its `Game` impl) so both
//! entry points stay thin: `main.rs` (native window, filesystem saves,
//! optional editor) and `web_entry.rs` (wasm-bindgen start: fetch assets,
//! then the same `run_game`). This split also keeps `editor_integration`
//! out of the library — editor wiring lives in `main.rs` only.

mod achievements;
mod constants;
mod drawing;
mod effects;
mod gameplay;
#[cfg(test)]
mod gameplay_tests;
mod menu;
mod spawning;
mod types;

#[cfg(target_arch = "wasm32")]
mod web_entry;

use engine_core::prelude::*;
use constants::*;
use spawning::*;
use types::*;

pub use types::SnakeGame;

/// The shared `GameConfig` for every target. Entry points add their own
/// platform extras on top (native: save paths anchored to the game dir;
/// web: nothing — no save paths means in-memory achievements and default
/// input bindings).
///
/// `asset_base` must be an ANCHORED base: native callers pass an absolute
/// path (`main.rs` derives it from `game_root!()` so the cwd never
/// matters); the web entry passes the deploy URL base. Passing a bare
/// relative path like `"assets"` would silently resolve against the
/// current working directory.
pub fn game_config(asset_base: &str) -> GameConfig {
    GameConfig::new("Insiculous Snake")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        .with_asset_base_path(asset_base)
}

impl Game for SnakeGame {
    fn init(&mut self, ctx: &mut GameContext) {
        // Resolve against the configured asset base so the same relative
        // path works natively (game dir) and on the web (VFS keys).
        let font_path = std::path::Path::new(ctx.assets.base_path()).join("fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        achievements::register_all(ctx.achievements);

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.tex_id = tex.id;

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        self.background = Some(spawn_background(
            ctx.world, tex.id, theme.bg_color, Vec2::new(WIN_W, WIN_H)));
        self.walls = spawn_walls(ctx.world, tex.id, theme.structure_color);

        // The snake and food spawn fresh on every `start_game()`. Build the
        // deforming grid backdrop now so it exists before the first run.
        self.grid = Some(default_playfield_grid(&theme));
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frame_count = self.frame_count.wrapping_add(1);

        match self.state.clone() {
            GameState::TitleScreen { selection } => self.update_title_input(ctx, selection),
            GameState::ModeSelect { selection } => self.update_mode_select_input(ctx, selection),
            GameState::Achievements => self.update_achievements_input(ctx),
            _ => self.update_gameplay(ctx),
        }

        self.update_entity_visibility(ctx);
        self.draw_ui(ctx);
    }
}
