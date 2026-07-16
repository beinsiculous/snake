//! Pure grid rules for Snake: direction buffering, single-snake stepping,
//! versus resolution, food placement, spawn layout, and tick timing.
//!
//! Every function here is plain data in, plain data out — no entities, no
//! `GameContext` — so all of Snake's rules stay headless-testable.

use std::collections::VecDeque;

use engine_core::prelude::*;
use glam::IVec2;

use crate::constants::*;
use crate::spawning::in_bounds;
use crate::types::{DeathCause, Direction, GameResult};

/// What one single-player grid step did. The caller turns this into
/// entities/effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepOutcome {
    Moved,
    /// The head landed on the food at this cell; the snake grew by one.
    Ate(IVec2),
    Died(DeathCause),
}

/// Per-snake result of one versus tick, computed from the pre-step board so
/// both snakes resolve against the same starting positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VersusStep {
    /// Survived and advanced into this new head cell.
    Moved(IVec2),
    /// Survived and ate the food at this new head cell; the snake grew.
    Ate(IVec2),
    Died(DeathCause),
}

/// Pop the first applicable queued turn. Turns equal or opposite to the
/// current heading are discarded (a 180 into your own neck is never legal),
/// and validation happens at apply time, so two buffered turns can never
/// combine into a reversal.
pub(crate) fn next_direction(current: Direction, queue: &mut VecDeque<Direction>) -> Direction {
    while let Some(d) = queue.pop_front() {
        if d != current && d != current.opposite() {
            return d;
        }
    }
    current
}

/// Advance the snake one cell in `direction`. `foods` are the live food
/// cells; `wrap` teleports edge exits to the far side instead of killing.
/// Mutates `cells` only on a survivable step.
pub(crate) fn step_snake(
    cells: &mut VecDeque<IVec2>,
    direction: Direction,
    foods: &[IVec2],
    wrap: bool,
) -> StepOutcome {
    let Some(new_head) = step_head(cells, direction, wrap) else {
        return StepOutcome::Died(DeathCause::Wall);
    };

    let eating = foods.contains(&new_head);
    // The tail cell vacates this step unless we grow, so stepping into it is
    // legal — exclude it from the self-bite check.
    if occupies(cells, eating, new_head) {
        return StepOutcome::Died(DeathCause::SelfBite);
    }

    cells.push_front(new_head);
    if eating {
        StepOutcome::Ate(new_head)
    } else {
        cells.pop_back();
        StepOutcome::Moved
    }
}

/// The cell a head moves into, or `None` when it leaves the field and walls
/// don't wrap.
fn step_head(cells: &VecDeque<IVec2>, direction: Direction, wrap: bool) -> Option<IVec2> {
    let head = *cells.front().expect("snake always has a head");
    let mut new_head = head + direction.delta();
    if wrap {
        new_head.x = new_head.x.rem_euclid(GRID_COLS);
        new_head.y = new_head.y.rem_euclid(GRID_ROWS);
        Some(new_head)
    } else if in_bounds(new_head) {
        Some(new_head)
    } else {
        None
    }
}

/// True if `cell` is a live body cell of a snake this tick. A snake that is
/// not eating vacates its tail, so that cell is excluded.
fn occupies(cells: &VecDeque<IVec2>, eating: bool, cell: IVec2) -> bool {
    let body_len = if eating { cells.len() } else { cells.len() - 1 };
    cells.iter().take(body_len).any(|&c| c == cell)
}

/// Resolve one versus tick for both snakes from their pre-step state. A head
/// dies if it leaves the field (walls don't wrap), both heads target the same
/// cell (head-on), or it lands on any live body cell of either snake — each
/// body excluding that snake's vacating tail when it isn't eating. No state is
/// mutated: the caller advances only the survivors.
pub(crate) fn resolve_versus_step(
    cells: [&VecDeque<IVec2>; 2],
    directions: [Direction; 2],
    foods: &[IVec2],
    wrap: bool,
) -> [VersusStep; 2] {
    let heads = [
        step_head(cells[0], directions[0], wrap),
        step_head(cells[1], directions[1], wrap),
    ];
    let eating = [
        heads[0].is_some_and(|h| foods.contains(&h)),
        heads[1].is_some_and(|h| foods.contains(&h)),
    ];

    let mut result = [VersusStep::Moved(IVec2::ZERO); 2];
    for i in 0..2 {
        let j = 1 - i;
        let Some(head) = heads[i] else {
            result[i] = VersusStep::Died(DeathCause::Wall);
            continue;
        };
        // Both heads into the same cell: they meet mid-air, both die.
        if heads[j] == Some(head) {
            result[i] = VersusStep::Died(DeathCause::HeadOn);
        } else if occupies(cells[i], eating[i], head) {
            result[i] = VersusStep::Died(DeathCause::SelfBite);
        } else if occupies(cells[j], eating[j], head) {
            // Covers the swap case too: each old head is still a body cell of
            // the other snake, so passing through each other is fatal.
            result[i] = VersusStep::Died(DeathCause::OtherSnake);
        } else if eating[i] {
            result[i] = VersusStep::Ate(head);
        } else {
            result[i] = VersusStep::Moved(head);
        }
    }
    result
}

/// Round outcome from a resolved versus tick: `None` while both snakes live,
/// otherwise the first-death result (lone survivor wins, simultaneous = draw).
pub(crate) fn versus_result(steps: &[VersusStep; 2]) -> Option<GameResult> {
    let cause = |step: &VersusStep| match step {
        VersusStep::Died(c) => Some(*c),
        _ => None,
    };
    match (cause(&steps[0]), cause(&steps[1])) {
        (Some(_), Some(_)) => Some(GameResult::Draw),
        (Some(c), None) => Some(GameResult::Winner { player: 2, loser_cause: c }),
        (None, Some(c)) => Some(GameResult::Winner { player: 1, loser_cause: c }),
        (None, None) => None,
    }
}

/// Head cell and heading for versus snake `index`: player 1 starts lower-left
/// heading right, player 2 upper-right heading left — disjoint rows so they
/// never spawn into a head-on.
pub(crate) fn versus_spawn(index: usize) -> (IVec2, Direction) {
    match index {
        0 => (IVec2::new(GRID_COLS / 4, GRID_ROWS / 3), Direction::Right),
        _ => (IVec2::new(3 * GRID_COLS / 4, 2 * GRID_ROWS / 3), Direction::Left),
    }
}

/// The starting body cells for a snake whose head is at `head` moving
/// `direction`: head first, the rest trailing opposite the heading.
pub(crate) fn starting_body(head: IVec2, direction: Direction, length: usize) -> Vec<IVec2> {
    let step = direction.delta();
    (0..length as i32)
        .map(|i| IVec2::new(head.x - step.x * i, head.y - step.y * i))
        .collect()
}

/// Deterministic food placement: hash the seed to a starting cell, then
/// linear-probe forward to the first free cell. Returns `None` only when the
/// board is completely occupied.
pub(crate) fn place_food(occupied: &[IVec2], seed: u32) -> Option<IVec2> {
    let total = (GRID_COLS * GRID_ROWS) as u32;
    let start = hash_u32(seed) % total;
    (0..total).map(|i| (start + i) % total).find_map(|idx| {
        let cell = IVec2::new((idx % GRID_COLS as u32) as i32, (idx / GRID_COLS as u32) as i32);
        (!occupied.contains(&cell)).then_some(cell)
    })
}

/// Seconds per grid step. Insane-family modes start faster and accelerate
/// with every food eaten, down to a floor.
pub(crate) fn tick_interval(mode: ChaosMode, foods_eaten: u32) -> f32 {
    if mode.is_insane() {
        (INSANE_TICK - foods_eaten as f32 * INSANE_TICK_STEP).max(INSANE_TICK_MIN)
    } else {
        NORMAL_TICK
    }
}

/// Ridiculous-family modes keep two pellets on the board.
pub(crate) fn food_count(mode: ChaosMode) -> usize {
    if mode.is_ridiculous() { 2 } else { 1 }
}

/// Wrap-around walls are the other half of the Ridiculous buff.
pub(crate) fn walls_wrap(mode: ChaosMode) -> bool {
    mode.is_ridiculous()
}
