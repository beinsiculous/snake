//! Headless tests for the snake rules: pure grid math only — no window,
//! no GPU, no physics.

use std::collections::VecDeque;

use engine_core::prelude::*;
use glam::IVec2;

use crate::constants::*;
use crate::gameplay::{
    food_count, next_direction, place_food, step_snake, tick_interval, walls_wrap, StepOutcome,
};
use crate::menu::mode_hint;
use crate::types::{DeathCause, Direction};

fn snake(cells: &[(i32, i32)]) -> VecDeque<IVec2> {
    cells.iter().map(|&(x, y)| IVec2::new(x, y)).collect()
}

// --- Directions and input buffering ---

#[test]
fn opposite_pairs_are_symmetric() {
    for dir in [Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
        assert_eq!(dir.opposite().opposite(), dir);
        assert_ne!(dir.opposite(), dir);
    }
}

#[test]
fn deltas_are_unit_steps() {
    for dir in [Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
        let d = dir.delta();
        assert_eq!(d.x.abs() + d.y.abs(), 1, "{dir:?} must move exactly one cell");
        assert_eq!(d + dir.opposite().delta(), IVec2::ZERO);
    }
}

#[test]
fn next_direction_applies_a_legal_turn() {
    let mut queue = VecDeque::from([Direction::Up]);
    assert_eq!(next_direction(Direction::Right, &mut queue), Direction::Up);
    assert!(queue.is_empty());
}

#[test]
fn next_direction_rejects_reversal() {
    let mut queue = VecDeque::from([Direction::Left]);
    assert_eq!(next_direction(Direction::Right, &mut queue), Direction::Right);
}

#[test]
fn next_direction_skips_redundant_then_takes_next() {
    // Holding Right then tapping Down: the redundant Right is discarded.
    let mut queue = VecDeque::from([Direction::Right, Direction::Down]);
    assert_eq!(next_direction(Direction::Right, &mut queue), Direction::Down);
}

#[test]
fn buffered_double_turn_cannot_reverse() {
    // Moving Right, player taps Up then Left within one tick: Up applies
    // now, Left waits — and Left is legal *after* the Up step, never as a
    // straight 180 from Right.
    let mut queue = VecDeque::from([Direction::Up, Direction::Left]);
    let first = next_direction(Direction::Right, &mut queue);
    assert_eq!(first, Direction::Up);
    let second = next_direction(first, &mut queue);
    assert_eq!(second, Direction::Left, "perpendicular follow-up turn is legal");

    // The dangerous order: skip-scan must not let a queued reversal through.
    let mut queue = VecDeque::from([Direction::Right, Direction::Left]);
    assert_eq!(next_direction(Direction::Right, &mut queue), Direction::Right);
}

// --- step_snake ---

#[test]
fn step_moves_head_and_tail_follows() {
    let mut cells = snake(&[(5, 5), (4, 5), (3, 5)]);
    let out = step_snake(&mut cells, Direction::Right, &[], false);
    assert_eq!(out, StepOutcome::Moved);
    assert_eq!(cells, snake(&[(6, 5), (5, 5), (4, 5)]));
}

#[test]
fn eating_grows_by_one_and_keeps_the_tail() {
    let mut cells = snake(&[(5, 5), (4, 5), (3, 5)]);
    let food = IVec2::new(6, 5);
    let out = step_snake(&mut cells, Direction::Right, &[food], false);
    assert_eq!(out, StepOutcome::Ate(food));
    assert_eq!(cells, snake(&[(6, 5), (5, 5), (4, 5), (3, 5)]));
}

#[test]
fn stepping_into_the_vacating_tail_cell_is_not_death() {
    // A 2x2 loop: the head re-enters the tail cell the same tick the tail
    // leaves it. Classic snake rules allow this.
    let mut cells = snake(&[(5, 6), (4, 6), (4, 5), (5, 5)]);
    let out = step_snake(&mut cells, Direction::Down, &[], false);
    assert_eq!(out, StepOutcome::Moved);
    assert_eq!(cells.front(), Some(&IVec2::new(5, 5)));
}

#[test]
fn stepping_into_the_tail_cell_while_eating_is_death() {
    // Same loop, but food on the re-entry cell: the tail does NOT vacate on
    // a growth tick, so the bite is real.
    let mut cells = snake(&[(5, 6), (4, 6), (4, 5), (5, 5)]);
    let food = IVec2::new(5, 5);
    let out = step_snake(&mut cells, Direction::Down, &[food], false);
    assert_eq!(out, StepOutcome::Died(DeathCause::SelfBite));
}

#[test]
fn biting_the_body_is_death_and_leaves_cells_untouched() {
    let mut cells = snake(&[(5, 5), (4, 5), (4, 6), (5, 6), (6, 6), (6, 5)]);
    let before = cells.clone();
    // Head at (5,5) turning up into (5,6), squarely on the body.
    let out = step_snake(&mut cells, Direction::Up, &[], false);
    assert_eq!(out, StepOutcome::Died(DeathCause::SelfBite));
    assert_eq!(cells, before, "a fatal step must not corrupt the body");
}

#[test]
fn leaving_the_field_is_death_without_wrap() {
    let mut cells = snake(&[(GRID_COLS - 1, 5), (GRID_COLS - 2, 5)]);
    let out = step_snake(&mut cells, Direction::Right, &[], false);
    assert_eq!(out, StepOutcome::Died(DeathCause::Wall));
}

#[test]
fn leaving_the_field_wraps_to_the_far_side_with_wrap() {
    let mut cells = snake(&[(GRID_COLS - 1, 5), (GRID_COLS - 2, 5)]);
    let out = step_snake(&mut cells, Direction::Right, &[], true);
    assert_eq!(out, StepOutcome::Moved);
    assert_eq!(cells.front(), Some(&IVec2::new(0, 5)));

    let mut cells = snake(&[(3, 0), (3, 1)]);
    let out = step_snake(&mut cells, Direction::Down, &[], true);
    assert_eq!(out, StepOutcome::Moved);
    assert_eq!(cells.front(), Some(&IVec2::new(3, GRID_ROWS - 1)));
}

// --- Food placement ---

#[test]
fn place_food_avoids_every_occupied_cell() {
    // Occupy everything except one corner; every seed must find that cell.
    let mut occupied = Vec::new();
    for x in 0..GRID_COLS {
        for y in 0..GRID_ROWS {
            if (x, y) != (GRID_COLS - 1, GRID_ROWS - 1) {
                occupied.push(IVec2::new(x, y));
            }
        }
    }
    for seed in 0..50 {
        assert_eq!(
            place_food(&occupied, seed),
            Some(IVec2::new(GRID_COLS - 1, GRID_ROWS - 1))
        );
    }
}

#[test]
fn place_food_returns_none_on_a_full_board() {
    let occupied: Vec<IVec2> = (0..GRID_COLS)
        .flat_map(|x| (0..GRID_ROWS).map(move |y| IVec2::new(x, y)))
        .collect();
    assert_eq!(place_food(&occupied, 7), None);
}

#[test]
fn place_food_lands_in_bounds_and_off_the_snake() {
    let occupied: Vec<IVec2> = snake(&[(5, 5), (4, 5), (3, 5)]).into();
    for seed in 0..200 {
        let cell = place_food(&occupied, seed).expect("plenty of room");
        assert!(crate::spawning::in_bounds(cell), "seed {seed} left the field: {cell}");
        assert!(!occupied.contains(&cell), "seed {seed} landed on the snake");
    }
}

// --- Chaos mode rules ---

#[test]
fn insane_family_ticks_faster_than_normal() {
    assert!(tick_interval(ChaosMode::Insane, 0) < tick_interval(ChaosMode::Normal, 0));
    assert!(tick_interval(ChaosMode::Insiculous, 0) < tick_interval(ChaosMode::Normal, 0));
    assert_eq!(tick_interval(ChaosMode::Ridiculous, 0), tick_interval(ChaosMode::Normal, 0));
}

#[test]
fn insane_tick_shrinks_per_food_and_clamps() {
    assert!(tick_interval(ChaosMode::Insane, 5) < tick_interval(ChaosMode::Insane, 0));
    assert_eq!(tick_interval(ChaosMode::Insane, 10_000), INSANE_TICK_MIN);
    // Normal mode never accelerates.
    assert_eq!(tick_interval(ChaosMode::Normal, 10_000), NORMAL_TICK);
}

#[test]
fn ridiculous_family_gets_double_food_and_wrapping_walls() {
    assert_eq!(food_count(ChaosMode::Normal), 1);
    assert_eq!(food_count(ChaosMode::Insane), 1);
    assert_eq!(food_count(ChaosMode::Ridiculous), 2);
    assert_eq!(food_count(ChaosMode::Insiculous), 2);

    assert!(!walls_wrap(ChaosMode::Normal));
    assert!(!walls_wrap(ChaosMode::Insane));
    assert!(walls_wrap(ChaosMode::Ridiculous));
    assert!(walls_wrap(ChaosMode::Insiculous));
}

#[test]
fn every_mode_has_a_hint() {
    for mode in ChaosMode::ALL {
        assert!(!mode_hint(mode).is_empty());
    }
}

// --- Start layout sanity ---

#[test]
fn starting_snake_fits_the_field_heading_right() {
    // start_game places the head mid-field with the body trailing left;
    // the whole starting body must be in bounds with marching room.
    let head = IVec2::new(GRID_COLS / 2, GRID_ROWS / 2);
    for i in 0..START_LENGTH as i32 {
        assert!(crate::spawning::in_bounds(IVec2::new(head.x - i, head.y)));
    }
    assert!(head.x + 3 < GRID_COLS, "room to move before the first turn");
}
