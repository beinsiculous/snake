//! Bratdog — game crate: snake, re-skinned as Frank the bratwurst dachshund.
//!
//! The library owns the whole game (`SnakeGame` + its `Game` impl) so both
//! entry points stay thin: `main.rs` (native window, filesystem saves,
//! optional editor) and `web_entry.rs` (wasm-bindgen start: fetch assets,
//! then the same `run_game`). This split also keeps `editor_integration`
//! behind the `editor` feature in both entry points — `main.rs` for the native
//! window, `web_entry.rs` for the browser's editor bundle.

mod achievements;
mod body;
mod constants;
mod drawing;
mod effects;
#[cfg(test)]
mod flow_tests;
mod gameplay;
#[cfg(test)]
mod gameplay_tests;
mod menu;
mod spawning;
#[cfg(test)]
mod test_support;
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
fn load_sheet(assets: &mut AssetManager, spec: &SheetSpec) -> SpriteSheet {
    assets
        .load_sprite_sheet(spec.path)
        .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path))
}

pub fn game_config(asset_base: &str) -> GameConfig {
    GameConfig::new("Bratdog")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        .with_pixel_snap(true)
        .with_startup_splashes(STARTUP_CARDS)
        .with_window_icon(WINDOW_ICON)
        .with_asset_base_path(asset_base)
}

impl Game for SnakeGame {
    fn register_achievements(&self, achievements: &mut AchievementManager, _strings: &Strings) {
        achievements::register_all(achievements);
    }

    fn init(&mut self, ctx: &mut GameContext) {
        // Resolve against the configured asset base so the same relative
        // path works natively (game dir) and on the web (VFS keys).
        let font_path = std::path::Path::new(ctx.assets.base_path()).join("fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.sheets.white = tex.id;

        // Every sheet's path and cell is in `constants.rs`'s sheets block; each PNG and
        // its `.sheet.ron` sidecar is a synced copy under `assets/sprites/`.
        self.sheets.frank = load_sheet(ctx.assets, &FRANK);
        self.sheets.frank_player_two = load_sheet(ctx.assets, &FRANK_PLAYER_TWO);
        self.sheets.pretzel = load_sheet(ctx.assets, &PRETZEL);
        self.sheets.cheese_bite = load_sheet(ctx.assets, &CHEESE_BITE);
        self.sheets.bacon_bone = load_sheet(ctx.assets, &BACON_BONE);
        self.sheets.kitchen_floor = load_sheet(ctx.assets, &KITCHEN_FLOOR);
        self.sheets.kitchen_wall = load_sheet(ctx.assets, &KITCHEN_WALL);

        // The kitchen, its wall and the grid over them stay up under the menus. The dogs
        // and the snacks spawn fresh on every `start_game()`.
        self.floor = Some(spawn_floor(ctx.world, &self.sheets.kitchen_floor));
        self.wall = Some(spawn_wall(ctx.world, &self.sheets.kitchen_wall));
        self.backdrop = Some(spawn_backdrop(ctx.world, &ChaosTheme::for_mode(self.chaos_mode)));
        self.apply_theme(ctx.world);
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
