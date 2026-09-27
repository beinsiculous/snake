//! Shared test fixtures.
//!
//! The synced sheets are read back through the engine's own GPU-free load path, so a
//! test can check the game's clip names against the committed art without a window, a
//! GPU or a `GameContext`. The fixtures also drive the whole game through the engine's
//! harness: the real frame, with no window.

use std::path::{Path, PathBuf};

use engine_core::assets::sprite_sheet::{prepare_sheet, PreparedSheet};
use engine_core::prelude::*;
use engine_core::test_support::GameHarness;
use glam::IVec2;

use crate::types::*;

pub(crate) const FRAME: f32 = 1.0 / 60.0;

/// The synced art's directory, anchored to the crate so the working directory never
/// matters.
fn asset_base() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// A synced sheet read through the engine's own GPU-free load path.
pub(crate) fn sidecar(spec: &SheetSpec) -> PreparedSheet {
    prepare_sheet(&asset_base(), spec.path)
        .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path))
}

/// The title screen's world: the game's own config over the synced art, with no save
/// paths so achievements and scores stay in memory, and one frame run so `init` has
/// loaded the sheets and spawned the kitchen.
pub(crate) fn title_harness() -> GameHarness<SnakeGame> {
    let base = asset_base();
    let config = crate::game_config(base.to_str().expect("the asset path is UTF-8"));
    let mut harness = GameHarness::new(SnakeGame::default(), config);
    harness.step(FRAME, &[]);
    harness
}

/// A match started as the mode select does it, in `mode` and `chaos`, with its pellets
/// cleared so nothing is eaten unless a test puts a snack in the way.
pub(crate) fn harness(mode: GameMode, chaos: ChaosMode) -> GameHarness<SnakeGame> {
    let mut harness = title_harness();
    harness.context(|game, ctx| {
        game.mode = mode;
        game.chaos_mode = chaos;
        ctx.chaos_mode = chaos;
        game.start_game(ctx);
        for food in game.foods.drain(..) {
            ctx.world.remove_entity(&food.entity).ok();
        }
    });
    harness
}

/// Put snake `index` on `cells`, head first, heading `direction`, with nothing buffered.
pub(crate) fn place_dog(harness: &mut GameHarness<SnakeGame>, index: usize, cells: &[IVec2], direction: Direction) {
    harness.context(|game, ctx| {
        let snake = &mut game.snakes[index];
        snake.cells = cells.iter().copied().collect();
        snake.direction = direction;
        snake.input_queue.clear();
        game.retile_dogs(ctx.world);
    });
}

/// Step until the next grid tick has run, and say how many frames that took.
pub(crate) fn step_one_tick(harness: &mut GameHarness<SnakeGame>) -> usize {
    let head = harness.game().snakes[0].cells.front().copied();
    for frame in 1..=60 {
        harness.step(FRAME, &[]);
        let game = harness.game();
        if game.snakes[0].cells.front().copied() != head || game.state != GameState::Playing {
            return frame;
        }
    }
    panic!("no tick ran in a second");
}

/// The tile a dog's map holds at grid cell `cell`.
pub(crate) fn dog_tile(harness: &GameHarness<SnakeGame>, index: usize, cell: IVec2) -> u32 {
    let map = harness.world().get::<Tilemap>(harness.game().snakes[index].map).expect("a dog has a map");
    map.tiles[(crate::spawning::map_row(cell.y) * map.width + cell.x as u32) as usize]
}

/// The tile value a clip shows `elapsed` seconds in, from the loaded sheet.
pub(crate) fn clip_tile(sheet: &SpriteSheet, clip: &str, elapsed: f32) -> u32 {
    crate::spawning::clip_cell(sheet, clip, elapsed).expect("the sheet declares the clip") + 1
}
