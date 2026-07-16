//! Headless tests for the snake rules: pure grid math only — no window,
//! no GPU, no physics.

use std::collections::VecDeque;

use engine_core::prelude::*;
use glam::IVec2;

use crate::constants::*;
use crate::gameplay::{
    food_count, next_direction, place_food, resolve_versus_step, starting_body, step_snake,
    tick_interval, versus_result, versus_spawn, walls_wrap, StepOutcome, VersusStep,
};
use crate::menu::mode_hint;
use crate::types::{DeathCause, Direction, GameResult};

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

// --- Versus resolution ---

/// Resolve a versus tick with no food on an empty (non-wrapping) board.
fn versus(
    a: &[(i32, i32)],
    dir_a: Direction,
    b: &[(i32, i32)],
    dir_b: Direction,
) -> [VersusStep; 2] {
    resolve_versus_step([&snake(a), &snake(b)], [dir_a, dir_b], &[], false)
}

#[test]
fn moving_into_other_snakes_body_is_fatal() {
    // A steps into (6,5), a live middle cell of B (B's tail (6,6) vacates).
    let steps = versus(
        &[(5, 5), (4, 5), (3, 5)], Direction::Right,
        &[(6, 4), (6, 5), (6, 6)], Direction::Down,
    );
    assert_eq!(steps[0], VersusStep::Died(DeathCause::OtherSnake));
    assert_eq!(steps[1], VersusStep::Moved(IVec2::new(6, 3)), "B moves clear");
}

#[test]
fn head_on_same_cell_kills_both_and_draws() {
    // Both heads target (5,5) from opposite sides.
    let steps = versus(
        &[(4, 5), (3, 5), (2, 5)], Direction::Right,
        &[(6, 5), (7, 5), (8, 5)], Direction::Left,
    );
    assert_eq!(steps[0], VersusStep::Died(DeathCause::HeadOn));
    assert_eq!(steps[1], VersusStep::Died(DeathCause::HeadOn));
    assert_eq!(versus_result(&steps), Some(GameResult::Draw));
}

#[test]
fn swapping_heads_through_each_other_kills_both() {
    // Adjacent, facing each other: each head moves onto the other's old head
    // cell (still a body cell), so both die even though it isn't a head-on.
    let steps = versus(
        &[(5, 5), (4, 5), (3, 5)], Direction::Right,
        &[(6, 5), (7, 5), (8, 5)], Direction::Left,
    );
    assert_eq!(steps[0], VersusStep::Died(DeathCause::OtherSnake));
    assert_eq!(steps[1], VersusStep::Died(DeathCause::OtherSnake));
    assert_eq!(versus_result(&steps), Some(GameResult::Draw));
}

#[test]
fn other_snakes_vacating_tail_cell_is_safe() {
    // B (head at (10,7)) moves up, vacating its tail (10,5); A steps onto that
    // freed cell the very same tick, which is legal.
    let steps = versus(
        &[(9, 5), (8, 5), (7, 5)], Direction::Right,
        &[(10, 7), (10, 6), (10, 5)], Direction::Up,
    );
    assert_eq!(steps[0], VersusStep::Moved(IVec2::new(10, 5)));
    assert_eq!(steps[1], VersusStep::Moved(IVec2::new(10, 8)));
    assert_eq!(versus_result(&steps), None, "both alive: round continues");
}

#[test]
fn survivor_wins_when_only_one_dies() {
    // A runs off the right edge; B moves safely — player 2 wins.
    let steps = versus(
        &[(GRID_COLS - 1, 5), (GRID_COLS - 2, 5)], Direction::Right,
        &[(2, 2), (2, 3), (2, 4)], Direction::Down,
    );
    assert_eq!(steps[0], VersusStep::Died(DeathCause::Wall));
    assert_eq!(
        versus_result(&steps),
        Some(GameResult::Winner { player: 2, loser_cause: DeathCause::Wall })
    );

    // Mirror: only snake 1 dies — player 1 wins.
    let steps = versus(
        &[(2, 2), (2, 3), (2, 4)], Direction::Down,
        &[(GRID_COLS - 1, 5), (GRID_COLS - 2, 5)], Direction::Right,
    );
    assert_eq!(
        versus_result(&steps),
        Some(GameResult::Winner { player: 1, loser_cause: DeathCause::Wall })
    );
}

#[test]
fn versus_spawns_are_disjoint_and_in_bounds() {
    let (h0, d0) = versus_spawn(0);
    let (h1, d1) = versus_spawn(1);
    let body0 = starting_body(h0, d0, START_LENGTH);
    let body1 = starting_body(h1, d1, START_LENGTH);

    for cell in body0.iter().chain(body1.iter()) {
        assert!(crate::spawning::in_bounds(*cell), "spawn cell off the field: {cell}");
    }
    for a in &body0 {
        assert!(!body1.contains(a), "snakes share spawn cell {a}");
    }
    // Heads face opposite ways on different rows, never an instant head-on.
    assert_ne!(d0, d1);
    assert_ne!(h0.y, h1.y);
}

#[test]
fn versus_food_is_never_placed_on_either_snake() {
    let (h0, d0) = versus_spawn(0);
    let (h1, d1) = versus_spawn(1);
    let occupied: Vec<IVec2> = starting_body(h0, d0, START_LENGTH)
        .into_iter()
        .chain(starting_body(h1, d1, START_LENGTH))
        .collect();

    for seed in 0..200 {
        let cell = place_food(&occupied, seed).expect("plenty of room for a pellet");
        assert!(crate::spawning::in_bounds(cell), "seed {seed} left the field: {cell}");
        assert!(!occupied.contains(&cell), "seed {seed} landed on a snake: {cell}");
    }
}
