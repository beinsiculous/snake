use engine_core::prelude::*;

pub(crate) const WIN_W: f32 = 800.0;
pub(crate) const WIN_H: f32 = 600.0;

// --- Playfield grid ---
pub(crate) const GRID_COLS: i32 = 26;
pub(crate) const GRID_ROWS: i32 = 16;
pub(crate) const CELL_PX: f32 = 28.0;
/// The playfield sits slightly below window center to leave a HUD band on top.
pub(crate) const PLAYFIELD_OFFSET_Y: f32 = -10.0;
/// Thickness of the wall frame drawn around the playfield.
pub(crate) const WALL_THICKNESS: f32 = 6.0;

// --- Snake ---
pub(crate) const START_LENGTH: usize = 3;
/// Buffered turns waiting to be applied (classic two-turns-ahead feel).
pub(crate) const INPUT_QUEUE_CAP: usize = 2;
/// Sprites are drawn slightly smaller than the cell so segments read as
/// individual links instead of one solid bar.
pub(crate) const SEGMENT_PX: f32 = CELL_PX - 4.0;
pub(crate) const FOOD_PX: f32 = CELL_PX - 10.0;

// --- Tick timing (seconds per grid step) ---
pub(crate) const NORMAL_TICK: f32 = 0.14;
/// Insane mode: faster base tick...
pub(crate) const INSANE_TICK: f32 = 0.10;
/// ...that shrinks a little with every food eaten...
pub(crate) const INSANE_TICK_STEP: f32 = 0.002;
/// ...down to this floor.
pub(crate) const INSANE_TICK_MIN: f32 = 0.05;

// --- Scoring ---
pub(crate) const FOOD_POINTS: u32 = 10;

// --- Look ---
pub(crate) const FOOD_COLOR: Vec4 = Vec4::new(1.0, 0.35, 0.55, 1.0);
/// Player 2's snake tint in versus (player 1 wears the chaos-mode accent).
pub(crate) const SNAKE2_COLOR: Vec4 = Vec4::new(0.30, 0.90, 1.0, 1.0);
pub(crate) const HEAD_EMISSIVE: f32 = 1.6;
pub(crate) const BODY_EMISSIVE: f32 = 0.9;
pub(crate) const FOOD_EMISSIVE: f32 = 2.2;
pub(crate) const WALL_EMISSIVE: f32 = 0.7;
/// Wrap-around modes dim the walls — they're portals, not hazards.
pub(crate) const WALL_WRAP_ALPHA: f32 = 0.25;

// Radial impulses kicked into the spring-mass background grid.
pub(crate) const GRID_IMPULSE_EAT_STRENGTH: f32 = 220.0;
pub(crate) const GRID_IMPULSE_EAT_RADIUS: f32 = 80.0;
pub(crate) const GRID_IMPULSE_DEATH_STRENGTH: f32 = 700.0;
pub(crate) const GRID_IMPULSE_DEATH_RADIUS: f32 = 160.0;

// --- Achievements ---
pub(crate) const LENGTH_MILESTONES: [usize; 3] = [10, 20, 35];
/// Length that earns the per-mode "feast" achievement.
pub(crate) const FEAST_LENGTH: usize = 15;
/// Eating this soon after the previous food earns QUICK_SNACK.
pub(crate) const QUICK_SNACK_WINDOW: f32 = 1.5;

// --- the startup cards and the window icon ---------------------------------------
// Synced from deion_assets like every sheet (`assets/sprites/sync.list`).

/// The cards every Insiculous game opens on, in order: the studio's, then the
/// engine's. The engine shows them before `init` (`GameConfig::with_startup_splashes`).
pub(crate) const STARTUP_CARDS: [&str; 2] = [
    "sprites/ai_be_insiculous_320x192.png",
    "sprites/ai_insiculous_2d_maxwell_splash_320x192.png",
];
/// The engine's icon: the window's until the game draws one of its own.
pub(crate) const WINDOW_ICON: &str = "sprites/ai_insiculous_2d_maxwell_icon_64x64.png";

#[cfg(test)]
mod startup_card_tests {
    use super::*;
    use engine_core::{AssetConfig, AssetManager};

    #[test]
    fn the_startup_cards_are_the_studio_then_the_engine_on_their_contract_backdrops() {
        let assets_directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let config = crate::game_config(assets_directory.to_str().expect("the asset path is UTF-8"));
        assert_eq!(config.startup_splashes, STARTUP_CARDS, "the studio's card, then the engine's");
        assert_eq!(config.window_icon.as_deref(), Some(WINDOW_ICON));

        // The corners are BRANDING.md's named fills, which the engine letterboxes each
        // card in; a redrawn backdrop that drifts from the contract fails here.
        let assets = AssetManager::headless(AssetConfig::from(&config));
        for (path, size, corner) in [
            (STARTUP_CARDS[0], (320, 192), Some([0x14, 0x10, 0x1F, 0xFF])),
            (STARTUP_CARDS[1], (320, 192), Some([0x4A, 0x44, 0x58, 0xFF])),
            (WINDOW_ICON, (64, 64), None),
        ] {
            let image = assets.image_backdrop(path).unwrap_or_else(|| panic!("{path} is synced"));
            assert_eq!((image.size.x, image.size.y), size, "{path}");
            if let Some(corner) = corner {
                assert_eq!(image.corner.to_rgba8(), corner, "{path}");
            }
        }
    }
}
