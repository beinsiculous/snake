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
/// never spawn into a head-on. Player 2's start is player 1's turned half a turn
/// about the board's centre, so each has the same room to every wall; the board's
/// dimensions alone would not keep that fair (on 24 x 15, `3 * cols / 4` gives
/// player 2 one cell more to run than player 1).
pub(crate) fn versus_spawn(index: usize) -> (IVec2, Direction) {
    let first = IVec2::new(GRID_COLS / 4, GRID_ROWS / 3);
    match index {
        0 => (first, Direction::Right),
        _ => (IVec2::new(GRID_COLS - 1 - first.x, GRID_ROWS - 1 - first.y), Direction::Left),
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

// --- how a dog is drawn ---

/// Which way a body runs through a straight piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

/// The top or bottom edge of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerticalEdge {
    North,
    South,
}

/// The left or right edge of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HorizontalEdge {
    West,
    East,
}

/// Which of Frank's pieces one body cell shows. A head or a tail names the way its nose
/// or its tip points, its neck on the opposite edge; a corner names the two edges its
/// sausage leaves the cell by, as the sheet's clips do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Piece {
    Head(Direction),
    Tail(Direction),
    Straight(Axis),
    Corner(VerticalEdge, HorizontalEdge),
}

impl Piece {
    /// The sheet clip this piece draws — the names are the sidecar's.
    pub(crate) fn clip(self) -> &'static str {
        use Direction::*;
        match self {
            Piece::Head(Right) => "head_east",
            Piece::Head(Down) => "head_south",
            Piece::Head(Left) => "head_west",
            Piece::Head(Up) => "head_north",
            Piece::Tail(Right) => "tail_east",
            Piece::Tail(Down) => "tail_south",
            Piece::Tail(Left) => "tail_west",
            Piece::Tail(Up) => "tail_north",
            Piece::Straight(Axis::Horizontal) => "body_horizontal",
            Piece::Straight(Axis::Vertical) => "body_vertical",
            Piece::Corner(VerticalEdge::South, HorizontalEdge::West) => "corner_south_west",
            Piece::Corner(VerticalEdge::North, HorizontalEdge::West) => "corner_north_west",
            Piece::Corner(VerticalEdge::North, HorizontalEdge::East) => "corner_north_east",
            Piece::Corner(VerticalEdge::South, HorizontalEdge::East) => "corner_south_east",
        }
    }
}

/// The clip a dead dog's head plays, facing the way its head piece faces.
pub(crate) fn hurt_clip(facing: Direction) -> &'static str {
    match facing {
        Direction::Right => "hurt_east",
        Direction::Down => "hurt_south",
        Direction::Left => "hurt_west",
        Direction::Up => "hurt_north",
    }
}

/// The direction of one grid step from `from` to `to`, the short way round: a step
/// across a wrap seam — column 23 to column 0 — is one cell, not twenty-three, so a
/// body that wraps still draws its connectors toward the edge it left by. `None` for
/// two cells that are not neighbours.
pub(crate) fn step_direction(from: IVec2, to: IVec2) -> Option<Direction> {
    let mut delta = to - from;
    if delta.x.abs() == GRID_COLS - 1 {
        delta.x = -delta.x.signum();
    }
    if delta.y.abs() == GRID_ROWS - 1 {
        delta.y = -delta.y.signum();
    }
    match (delta.x, delta.y) {
        (1, 0) => Some(Direction::Right),
        (-1, 0) => Some(Direction::Left),
        (0, 1) => Some(Direction::Up),
        (0, -1) => Some(Direction::Down),
        _ => None,
    }
}

/// The piece `cell` shows, from its neighbours along the body: the one toward the head
/// (`None` for the head itself) and the one toward the tail (`None` for the tail). `None`
/// for a dog of one cell or for neighbours that do not touch `cell`.
pub(crate) fn piece_for(toward_head: Option<IVec2>, cell: IVec2, toward_tail: Option<IVec2>) -> Option<Piece> {
    let headward = toward_head.map(|neighbour| step_direction(cell, neighbour));
    let tailward = toward_tail.map(|neighbour| step_direction(cell, neighbour));
    match (headward, tailward) {
        (None, Some(Some(neck))) => Some(Piece::Head(neck.opposite())),
        (Some(Some(body)), None) => Some(Piece::Tail(body.opposite())),
        (Some(Some(first)), Some(Some(second))) if first == second.opposite() => {
            Some(Piece::Straight(match first {
                Direction::Left | Direction::Right => Axis::Horizontal,
                Direction::Up | Direction::Down => Axis::Vertical,
            }))
        }
        (Some(Some(first)), Some(Some(second))) => {
            let vertical = [first, second].into_iter().find_map(|direction| match direction {
                Direction::Up => Some(VerticalEdge::North),
                Direction::Down => Some(VerticalEdge::South),
                _ => None,
            })?;
            let horizontal = [first, second].into_iter().find_map(|direction| match direction {
                Direction::Left => Some(HorizontalEdge::West),
                Direction::Right => Some(HorizontalEdge::East),
                _ => None,
            })?;
            Some(Piece::Corner(vertical, horizontal))
        }
        _ => None,
    }
}

// --- the snacks ---

/// The three snacks a pellet can be. They score and grow the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Snack {
    Pretzel,
    CheeseBite,
    BaconBone,
}

impl Snack {
    pub(crate) const ALL: [Snack; 3] = [Snack::Pretzel, Snack::CheeseBite, Snack::BaconBone];
}

/// Salts a pellet's seed before the snack is read from it, so the snack does not follow
/// the hash `place_food` read the pellet's cell from.
const SNACK_SALT: u32 = 0x5EED_B0DE;

/// The snack a pellet placed from `seed` is. `place_food` takes the cell from the hash's
/// value modulo the board, and a board 24 wide is a multiple of 3, so the same value
/// modulo 3 would stripe the snacks by column; the snack is read instead from a salted
/// hash's high bits, which the multiplicative hash mixes best.
pub(crate) fn snack_for(seed: u32) -> Snack {
    Snack::ALL[((hash_u32(seed ^ SNACK_SALT) >> 16) % 3) as usize]
}
