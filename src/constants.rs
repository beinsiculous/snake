use engine_core::prelude::*;

use crate::gameplay::Snack;
use crate::types::Dog;

/// An opaque colour from its `0xRRGGBB` hex — the form DEION_STYLE.md § 4 records
/// every ramp in, so a constant here reads the same as the style guide's row.
const fn rgb(hex: u32) -> Vec4 {
    Vec4::new(
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
        1.0,
    )
}

pub(crate) const WIN_W: f32 = 800.0;
pub(crate) const WIN_H: f32 = 600.0;

// --- Playfield grid ---
// Bratdog is drawn at one art pixel per window pixel, pixel-snapped, on Frank's
// own 32 px pitch: a 24 x 15 grid is 768 x 480, and the 16 px kitchen wall around
// it makes an 800 x 512 arena that spans the window's width. Centred, it leaves a
// 44 px band above for the HUD and one below for the chaos banner.
pub(crate) const GRID_COLS: i32 = 24;
pub(crate) const GRID_ROWS: i32 = 15;
pub(crate) const CELL_PX: f32 = 32.0;
/// Where the arena's centre sits: on the window's.
pub(crate) const PLAYFIELD_OFFSET_Y: f32 = 0.0;
/// The kitchen wall's tile, and so the ring's thickness.
pub(crate) const WALL_TILE_PX: f32 = 16.0;
/// Wall tiles along one grid cell's edge.
pub(crate) const WALL_TILES_PER_CELL: i32 = 2;

// --- Snake ---
pub(crate) const START_LENGTH: usize = 3;
/// Buffered turns waiting to be applied (classic two-turns-ahead feel).
pub(crate) const INPUT_QUEUE_CAP: usize = 2;

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

// --- the sheets ------------------------------------------------------------------
// Every sheet is a synced copy (`assets/sprites/sync.list`). Nothing in this game has
// a collider — every rule is grid math — so no drawn box anchors anything: each
// spec's bounds are its whole cell and `sprite_offset()` is zero.

const DOG_CELL: Vec2 = Vec2::new(32.0, 32.0);
const SNACK_CELL: Vec2 = Vec2::new(32.0, 32.0);
const FLOOR_CELL: Vec2 = Vec2::new(32.0, 32.0);
const WALL_CELL: Vec2 = Vec2::new(16.0, 16.0);

const fn whole_cell(cell: Vec2) -> (Vec2, Vec2) {
    (Vec2::ZERO, cell)
}

/// Frank, player one: the overhead pieces a body is laid from, cell by cell.
pub(crate) const FRANK: SheetSpec = SheetSpec {
    path: "sprites/ai_frank.png",
    cell: DOG_CELL,
    bounds: whole_cell(DOG_CELL),
};
/// Player two: Frank recoloured, every cell's outline identical to his.
pub(crate) const FRANK_PLAYER_TWO: SheetSpec = SheetSpec {
    path: "sprites/ai_frank_player_two.png",
    cell: DOG_CELL,
    bounds: whole_cell(DOG_CELL),
};
pub(crate) const PRETZEL: SheetSpec = SheetSpec {
    path: "sprites/ai_bratdog_pretzel_32x32.png",
    cell: SNACK_CELL,
    bounds: whole_cell(SNACK_CELL),
};
pub(crate) const CHEESE_BITE: SheetSpec = SheetSpec {
    path: "sprites/ai_bratdog_cheese_bite_32x32.png",
    cell: SNACK_CELL,
    bounds: whole_cell(SNACK_CELL),
};
pub(crate) const BACON_BONE: SheetSpec = SheetSpec {
    path: "sprites/ai_bratdog_bacon_bone_32x32.png",
    cell: SNACK_CELL,
    bounds: whole_cell(SNACK_CELL),
};
pub(crate) const KITCHEN_FLOOR: SheetSpec = SheetSpec {
    path: "sprites/ai_kitchen_floor_32x32.png",
    cell: FLOOR_CELL,
    bounds: whole_cell(FLOOR_CELL),
};
pub(crate) const KITCHEN_WALL: SheetSpec = SheetSpec {
    path: "sprites/ai_kitchen_wall_16x16.png",
    cell: WALL_CELL,
    bounds: whole_cell(WALL_CELL),
};

// The clip names the game types — the contract with the sidecars.
pub(crate) const SNACK_IDLE: &str = "idle";
pub(crate) const SNACK_COLLECT: &str = "collect";
pub(crate) const FLOOR_CLIP: &str = "idle";

// --- draw depths -------------------------------------------------------------------
// One depth per sheet: the renderer draws each sheet's sprites as one batch, ordered by
// its lowest depth, and transparent texels write depth — so a sheet with entities both
// under and over another sheet's would punch holes in it. Nested from the floor up; every
// one stays under the particles the engine draws at 0.5. The snacks draw over the dogs:
// a collect plays on the cell the head has just entered, and under the head it would be
// hidden, while an idle snack stands on a free cell and is never under a dog.

pub(crate) const FLOOR_DEPTH: f32 = -2.0;
pub(crate) const WALL_DEPTH: f32 = -1.5;
pub(crate) const FRANK_DEPTH: f32 = -0.20;
pub(crate) const FRANK_PLAYER_TWO_DEPTH: f32 = -0.15;
pub(crate) const PRETZEL_DEPTH: f32 = 0.20;
pub(crate) const CHEESE_BITE_DEPTH: f32 = 0.25;
pub(crate) const BACON_BONE_DEPTH: f32 = 0.30;

/// What draws one dog or one snack: its sheet, its one draw depth, and the colour of
/// the burst it throws — a dog's when it dies, a snack's when it is eaten.
pub(crate) struct Art {
    pub(crate) spec: &'static SheetSpec,
    pub(crate) depth: f32,
    pub(crate) burst_color: Vec4,
}

pub(crate) fn dog_art(dog: Dog) -> Art {
    match dog {
        Dog::Frank => Art { spec: &FRANK, depth: FRANK_DEPTH, burst_color: FRANK_BURST_COLOR },
        Dog::PlayerTwo => Art {
            spec: &FRANK_PLAYER_TWO,
            depth: FRANK_PLAYER_TWO_DEPTH,
            burst_color: FRANK_PLAYER_TWO_BURST_COLOR,
        },
    }
}

pub(crate) fn snack_art(snack: Snack) -> Art {
    match snack {
        Snack::Pretzel => Art { spec: &PRETZEL, depth: PRETZEL_DEPTH, burst_color: PRETZEL_BURST_COLOR },
        Snack::CheeseBite => Art { spec: &CHEESE_BITE, depth: CHEESE_BITE_DEPTH, burst_color: CHEESE_BITE_BURST_COLOR },
        Snack::BaconBone => Art { spec: &BACON_BONE, depth: BACON_BONE_DEPTH, burst_color: BACON_BONE_BURST_COLOR },
    }
}

// --- Look ---
/// Each dog's death burst: the core of its sausage.
pub(crate) const FRANK_BURST_COLOR: Vec4 = rgb(0xD33F4C);
pub(crate) const FRANK_PLAYER_TWO_BURST_COLOR: Vec4 = rgb(0x59322B);
/// Each snack's eat burst: the colour of its body.
pub(crate) const PRETZEL_BURST_COLOR: Vec4 = rgb(0x9C6B3C);
pub(crate) const CHEESE_BITE_BURST_COLOR: Vec4 = rgb(0xE8B23C);
pub(crate) const BACON_BONE_BURST_COLOR: Vec4 = rgb(0xF4796F);

/// The backdrop grid's alpha, well under the preset's resting value: the lattice reads
/// over the kitchen floor without veiling it.
pub(crate) const BACKDROP_ALPHA: f32 = 0.25;

/// Collider outlines for the F1 overlay. Bratdog has no colliders, so the overlay draws
/// nothing; the key is kept for parity with the other games.
pub(crate) const DEBUG_COLLIDER_COLOR: Vec4 = Vec4::new(1.0, 0.2, 1.0, 0.9);
pub(crate) const DEBUG_COLLIDER_EMISSIVE: f32 = 2.0;

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
