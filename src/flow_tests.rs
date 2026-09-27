//! The match driven through the engine's harness over the synced art: the real frame,
//! with no window. Every test clears the board's pellets first, so a dog eats only a
//! snack the test puts in its way.

use engine_core::prelude::*;
use engine_core::test_support::{DrawCommand, GameHarness, InputEvent};
use glam::IVec2;

use crate::constants::*;
use crate::gameplay::{hurt_clip, piece_for, Snack};
use crate::spawning::{ring_clip, spawn_snack, RING_COLS, RING_ROWS};
use crate::test_support::*;
use crate::types::*;

fn cell(x: i32, y: i32) -> IVec2 {
    IVec2::new(x, y)
}

/// Every clip a dog's pieces and its hurt name.
fn dog_clips() -> Vec<&'static str> {
    let directions = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];
    let middle = cell(5, 5);
    let mut clips: Vec<&'static str> = Vec::new();
    for toward_head in directions {
        for toward_tail in directions {
            let head_side = Some(middle + toward_head.delta());
            let tail_side = Some(middle + toward_tail.delta());
            for piece in [piece_for(head_side, middle, tail_side), piece_for(None, middle, tail_side), piece_for(head_side, middle, None)]
                .into_iter()
                .flatten()
            {
                clips.push(piece.clip());
            }
        }
    }
    clips.extend(directions.map(hurt_clip));
    clips.sort_unstable();
    clips.dedup();
    clips
}

#[test]
fn every_clip_the_game_types_is_declared_by_its_sheet() {
    let dog_clips = dog_clips();
    assert_eq!(dog_clips.len(), 18, "fourteen pieces and four hurts");
    for spec in [&FRANK, &FRANK_PLAYER_TWO] {
        let sheet = sidecar(spec).sheet;
        for clip in &dog_clips {
            let declared = sheet.clips.iter().find(|(name, _)| name == clip);
            let declared = declared.unwrap_or_else(|| panic!("{} lacks {clip}", spec.path));
            assert_eq!(declared.1.looping, !clip.starts_with("hurt_"), "{} {clip}: a hurt plays once", spec.path);
        }
    }
    for spec in [&PRETZEL, &CHEESE_BITE, &BACON_BONE] {
        let clips = sidecar(spec).sheet.clips;
        let looping = |name: &str| clips.iter().find(|(clip, _)| clip == name).map(|(_, clip)| clip.looping);
        assert_eq!(looping(SNACK_IDLE), Some(true), "{}", spec.path);
        assert_eq!(looping(SNACK_COLLECT), Some(false), "{}: a collect plays once", spec.path);
    }
    let floor = sidecar(&KITCHEN_FLOOR).sheet;
    assert!(floor.clips.iter().any(|(name, _)| name == FLOOR_CLIP));
    let wall = sidecar(&KITCHEN_WALL).sheet;
    for open in [false, true] {
        for row in 0..RING_ROWS {
            for col in 0..RING_COLS {
                if let Some(clip) = ring_clip(col, row, open) {
                    assert!(wall.clips.iter().any(|(name, _)| name == clip), "the wall lacks {clip}");
                }
            }
        }
    }
}

#[test]
fn a_new_match_lays_each_dog_s_map_from_its_cells_and_nothing_else() {
    for mode in [GameMode::SinglePlayer, GameMode::TwoPlayerVersus] {
        let harness = harness(mode, ChaosMode::Normal);
        let game = harness.game();
        for (index, snake) in game.snakes.iter().enumerate() {
            let sheet = game.sheets.dog(snake.dog);
            let map = harness.world().get::<Tilemap>(snake.map).expect("a dog has a map");
            let drawn = map.tiles.iter().filter(|&&tile| tile != 0).count();
            assert_eq!(drawn, snake.cells.len(), "{mode:?} dog {index}: one tile per cell");
            for (position, &body_cell) in snake.cells.iter().enumerate() {
                let toward_head = position.checked_sub(1).map(|headward| snake.cells[headward]);
                let piece = piece_for(toward_head, body_cell, snake.cells.get(position + 1).copied()).expect("a piece");
                assert_eq!(dog_tile(&harness, index, body_cell), clip_tile(sheet, piece.clip(), 0.0), "{mode:?} {piece:?}");
            }
        }
        if mode == GameMode::TwoPlayerVersus {
            assert_eq!((game.snakes[0].dog, game.snakes[1].dog), (Dog::Frank, Dog::PlayerTwo));
        }
    }
}

#[test]
fn a_tick_empties_the_tail_cell_the_dog_left() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let old_tail = *harness.game().snakes[0].cells.back().expect("a tail");
    step_one_tick(&mut harness);
    assert!(!harness.game().snakes[0].cells.contains(&old_tail));
    assert_eq!(dog_tile(&harness, 0, old_tail), 0, "the cell the tail left is empty");
}

#[test]
fn the_head_always_shows_its_clip_at_the_play_clock_tick_frames_included() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Insane);
    let mut ticks_seen = 0;
    for _ in 0..40 {
        let head_before = harness.game().snakes[0].cells.front().copied();
        harness.step(FRAME, &[]);
        let game = harness.game();
        let snake = &game.snakes[0];
        ticks_seen += usize::from(snake.cells.front().copied() != head_before);
        let head = *snake.cells.front().expect("a head");
        let piece = piece_for(None, head, snake.cells.get(1).copied()).expect("a head piece");
        let expected = clip_tile(game.sheets.dog(snake.dog), piece.clip(), game.play_time);
        assert_eq!(dog_tile(&harness, 0, head), expected, "at play time {}", game.play_time);
    }
    assert!(ticks_seen >= 3, "the run crossed tick frames ({ticks_seen})");
}

#[test]
fn a_turn_into_the_wall_holds_a_hurt_head_that_still_joins_its_neck() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let top = GRID_ROWS - 1;
    let (head, neck, tail) = (cell(5, top), cell(4, top), cell(3, top));
    place_dog(&mut harness, 0, &[head, neck, tail], Direction::Right);
    harness.context(|game, _| game.snakes[0].input_queue.push_back(Direction::Up));
    step_one_tick(&mut harness);
    assert!(matches!(harness.game().state, GameState::GameOver { result: GameResult::Solo(DeathCause::Wall) }));
    assert_eq!(harness.game().snakes[0].direction, Direction::Up, "the turn was taken");
    assert_eq!(harness.game().snakes[0].cells.front(), Some(&head), "the fatal step moved nothing");

    // Half a second of game over: the three 100 ms hurt frames play out and the last holds.
    for _ in 0..30 {
        harness.step(FRAME, &[]);
    }
    let game = harness.game();
    let sheet = game.sheets.dog(Dog::Frank);
    assert_eq!(dog_tile(&harness, 0, head), clip_tile(sheet, "hurt_east", 10.0), "facing its neck, held");
    assert_ne!(dog_tile(&harness, 0, head), clip_tile(sheet, "hurt_east", 0.0), "past the first frame");
    assert_eq!(dog_tile(&harness, 0, tail), clip_tile(sheet, "tail_west", 0.0), "the tail has stopped");
}

#[test]
fn a_versus_head_on_kills_both_dogs_and_both_show_it() {
    let mut harness = harness(GameMode::TwoPlayerVersus, ChaosMode::Normal);
    place_dog(&mut harness, 0, &[cell(10, 7), cell(9, 7), cell(8, 7)], Direction::Right);
    place_dog(&mut harness, 1, &[cell(12, 7), cell(13, 7), cell(14, 7)], Direction::Left);
    step_one_tick(&mut harness);
    assert!(matches!(harness.game().state, GameState::GameOver { result: GameResult::Draw }));
    for _ in 0..30 {
        harness.step(FRAME, &[]);
    }
    let game = harness.game();
    assert!(game.snakes.iter().all(|snake| snake.death_time.is_some()));
    assert_eq!(dog_tile(&harness, 0, cell(10, 7)), clip_tile(game.sheets.dog(Dog::Frank), "hurt_east", 10.0));
    assert_eq!(dog_tile(&harness, 1, cell(12, 7)), clip_tile(game.sheets.dog(Dog::PlayerTwo), "hurt_west", 10.0));
}

#[test]
fn the_ring_is_open_in_a_wrap_mode_and_closed_otherwise() {
    let side = |harness: &GameHarness<SnakeGame>| {
        let map = harness.world().get::<Tilemap>(harness.game().wall.expect("a wall")).expect("a map");
        (map.tiles[7], map.tiles[0], map.tiles.iter().filter(|&&tile| tile != 0).count())
    };
    let ring_tiles = (2 * (RING_COLS + RING_ROWS) - 4) as usize;

    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Ridiculous);
    let wall = &harness.game().sheets.kitchen_wall;
    let (north_open, north, corner) =
        (clip_tile(wall, "north_open", 0.0), clip_tile(wall, "north", 0.0), clip_tile(wall, "corner_north_west", 0.0));
    assert_eq!(side(&harness), (north_open, corner, ring_tiles), "open, and never empty");

    harness.context(|game, ctx| {
        game.chaos_mode = ChaosMode::Normal;
        game.start_game(ctx);
    });
    assert_eq!(side(&harness), (north, corner, ring_tiles), "closed again");
}

/// Put a snack on `at`, in front of the dog.
fn lay_snack(harness: &mut GameHarness<SnakeGame>, at: IVec2, snack: Snack) {
    harness.context(|game, ctx| {
        let entity = spawn_snack(ctx.world, game.sheets.snack(snack), snack, at);
        game.foods.push(Food { cell: at, snack, entity });
    });
}

#[test]
fn an_eaten_snack_leaves_its_collect_over_the_dog_and_the_collect_ends_itself() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let ahead = *harness.game().snakes[0].cells.front().expect("a head") + Direction::Right.delta();
    lay_snack(&mut harness, ahead, Snack::CheeseBite);
    step_one_tick(&mut harness);
    assert_eq!(harness.game().snakes[0].foods_eaten, 1);

    let collect = harness.game().collects.first().expect("a collect").entity;
    let world = harness.world();
    let collect_depth = world.get::<Sprite>(collect).expect("drawn").depth;
    let dog_depth = world.get::<Tilemap>(harness.game().snakes[0].map).expect("a map").depth;
    assert!(collect_depth > dog_depth, "the collect draws over the head that ate it");
    assert_eq!(world.get::<Transform2D>(collect).expect("placed").position, crate::spawning::cell_to_world(ahead));

    for _ in 0..40 {
        harness.step(FRAME, &[]);
    }
    assert!(harness.world().get::<Sprite>(collect).is_none(), "four 100 ms frames, then gone");
}

#[test]
fn quitting_to_the_title_after_a_death_leaves_no_dog_and_no_collect() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let right = GRID_COLS - 1;
    place_dog(&mut harness, 0, &[cell(right - 1, 4), cell(right - 2, 4), cell(right - 3, 4)], Direction::Right);
    lay_snack(&mut harness, cell(right, 4), Snack::Pretzel);
    step_one_tick(&mut harness);
    step_one_tick(&mut harness);
    assert!(matches!(harness.game().state, GameState::GameOver { .. }));
    let (map, collect) = (harness.game().snakes[0].map, harness.game().collects[0].entity);
    assert!(harness.world().get::<Sprite>(collect).is_some(), "the collect is still playing");

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Escape)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::Escape)]);
    assert!(matches!(harness.game().state, GameState::TitleScreen { .. }));
    assert!(harness.world().get::<Tilemap>(map).is_none(), "the dog's map went with its match");
    assert!(harness.world().get::<Sprite>(collect).is_none(), "the collect went with it");
    assert!(harness.game().snakes.is_empty() && harness.game().collects.is_empty());
}

#[test]
fn the_hud_and_the_banner_draw_clear_of_the_ring() {
    let arena_half_height = RING_ROWS as f32 * WALL_TILE_PX / 2.0;
    let ring_top = WIN_H / 2.0 - (PLAYFIELD_OFFSET_Y + arena_half_height);
    let ring_bottom = WIN_H / 2.0 - (PLAYFIELD_OFFSET_Y - arena_half_height);
    for mode in [GameMode::SinglePlayer, GameMode::TwoPlayerVersus] {
        let mut harness = harness(mode, ChaosMode::Insane);
        harness.step(FRAME, &[]);
        let boxes: Vec<(String, f32, f32)> = harness
            .ui_commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { data, .. } => Some((data.text.clone(), data.position.y, data.position.y + data.height)),
                _ => None,
            })
            .collect();
        assert!(boxes.len() >= 3, "{mode:?}: the HUD's two labels and the banner are drawn as text");
        for (text, top, bottom) in boxes {
            assert!(bottom <= ring_top || top >= ring_bottom, "{mode:?}: '{text}' spans {top}..{bottom}, the ring {ring_top}..{ring_bottom}");
        }
    }
}

#[test]
fn each_sheet_draws_at_one_depth_nested_from_the_floor_up_under_the_particles() {
    let ladder = [FLOOR_DEPTH, WALL_DEPTH, FRANK_DEPTH, FRANK_PLAYER_TWO_DEPTH, PRETZEL_DEPTH, CHEESE_BITE_DEPTH, BACON_BONE_DEPTH];
    assert!(ladder.windows(2).all(|pair| pair[0] < pair[1]), "{ladder:?}");
    const { assert!(BACON_BONE_DEPTH < 0.5) };
}

#[test]
fn two_idle_dogs_start_mirrored_and_hit_their_walls_on_the_same_tick() {
    let mut harness = harness(GameMode::TwoPlayerVersus, ChaosMode::Normal);
    let (first, second) = (harness.game().snakes[0].cells.clone(), harness.game().snakes[1].cells.clone());
    for (one, other) in first.iter().zip(second.iter()) {
        assert_eq!(*other, cell(GRID_COLS - 1 - one.x, GRID_ROWS - 1 - one.y), "player 2 is player 1 turned about the centre");
    }
    for _ in 0..2000 {
        harness.step(FRAME, &[]);
        if harness.game().state != GameState::Playing {
            break;
        }
    }
    assert!(
        matches!(harness.game().state, GameState::GameOver { result: GameResult::Draw }),
        "neither dog is given a longer run to its wall"
    );
}

#[test]
fn a_fresh_pellet_never_lands_on_a_collect_still_playing() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let free_cell = cell(0, 0);
    harness.context(|game, ctx| {
        let dog: Vec<IVec2> = game.snakes[0].cells.iter().copied().collect();
        let sheet = game.sheets.snack(Snack::Pretzel);
        for x in 0..GRID_COLS {
            for y in 0..GRID_ROWS {
                let playing = cell(x, y);
                if playing != free_cell && !dog.contains(&playing) {
                    let entity = crate::spawning::spawn_collect(ctx.world, sheet, Snack::Pretzel, playing);
                    game.collects.push(Collect { entity, cell: playing });
                }
            }
        }
        game.spawn_missing_food(ctx.world);
    });
    let placed: Vec<IVec2> = harness.game().foods.iter().map(|food| food.cell).collect();
    assert_eq!(placed, vec![free_cell], "the only cell no dog and no collect covers");
}

#[test]
fn a_played_out_collect_is_forgotten_when_the_next_snack_is_eaten() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let head = *harness.game().snakes[0].cells.front().expect("a head");
    lay_snack(&mut harness, head + Direction::Right.delta(), Snack::BaconBone);
    step_one_tick(&mut harness);
    let first = harness.game().collects[0].entity;
    for _ in 0..40 {
        harness.step(FRAME, &[]);
    }
    for food in harness.game().foods.iter().map(|food| food.entity).collect::<Vec<_>>() {
        harness.world_mut().remove_entity(&food).ok();
    }
    harness.context(|game, _| game.foods.clear());
    let ahead = *harness.game().snakes[0].cells.front().expect("a head") + harness.game().snakes[0].direction.delta();
    lay_snack(&mut harness, ahead, Snack::Pretzel);
    step_one_tick(&mut harness);
    let collects: Vec<EntityId> = harness.game().collects.iter().map(|collect| collect.entity).collect();
    assert_eq!(collects.len(), 1, "only the collect still playing is kept");
    assert_ne!(collects[0], first);
}

#[test]
fn a_pellet_held_back_by_a_playing_collect_returns_once_the_collect_has_played_out() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let held_cell = cell(0, 0);
    let mut blockers = Vec::new();
    harness.context(|game, ctx| {
        let dog: Vec<IVec2> = game.snakes[0].cells.iter().copied().collect();
        let sheet = game.sheets.snack(Snack::Pretzel);
        for x in 0..GRID_COLS {
            for y in 0..GRID_ROWS {
                let covered = cell(x, y);
                if !dog.contains(&covered) {
                    let entity = crate::spawning::spawn_collect(ctx.world, sheet, Snack::Pretzel, covered);
                    game.collects.push(Collect { entity, cell: covered });
                    if covered != held_cell {
                        blockers.push(entity);
                    }
                }
            }
        }
    });
    harness.step(FRAME, &[]);
    assert!(harness.game().foods.is_empty(), "every free cell is under a playing collect");

    // Every collect but the one on the held cell has played out.
    for entity in blockers {
        harness.world_mut().remove_entity(&entity).ok();
    }
    harness.context(|game, ctx| {
        let held = game.collects.iter().find(|collect| collect.cell == held_cell).expect("held").entity;
        ctx.world.remove_entity(&held).ok();
    });
    harness.step(FRAME, &[]);
    assert_eq!(harness.game().foods.len(), 1, "the missing pellet is placed without another eat");
}
