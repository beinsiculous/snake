use std::collections::VecDeque;

use engine_core::prelude::*;
use glam::IVec2;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    ModeSelect { selection: u8 },
    Achievements,
    Playing,
    GameOver { result: GameResult },
}

/// How many snakes share the grid and where their input comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameMode {
    /// One snake driven by both players' controls (WASD + arrows + either pad).
    SinglePlayer,
    /// Two snakes on one grid; first death ends the round.
    TwoPlayerVersus,
}

/// What killed a snake — the game-over overlay names the culprit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeathCause {
    Wall,
    SelfBite,
    /// Ran into the other snake's body.
    OtherSnake,
    /// Both heads targeted the same cell — a mutual head-on.
    HeadOn,
}

/// The outcome of a finished round. Single-player rounds are always `Solo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameResult {
    /// A single-player death by the given cause.
    Solo(DeathCause),
    /// A versus round won by `player` (1-based); the loser died by `loser_cause`.
    Winner { player: u8, loser_cause: DeathCause },
    /// A versus round where both snakes died on the same tick.
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Grid delta for one step. Row 0 is the bottom row, so `Up` is +y in
    /// both grid and world space.
    pub(crate) fn delta(self) -> IVec2 {
        match self {
            Direction::Up => IVec2::new(0, 1),
            Direction::Down => IVec2::new(0, -1),
            Direction::Left => IVec2::new(-1, 0),
            Direction::Right => IVec2::new(1, 0),
        }
    }

    pub(crate) fn opposite(self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

/// A live food pellet: its grid cell plus the sprite showing it.
pub(crate) struct Food {
    pub(crate) cell: IVec2,
    pub(crate) entity: EntityId,
}

/// One snake's complete state. Single-player uses `snakes[0]`; versus adds a
/// second. `cells` is the source of truth — segment sprites are repositioned
/// from it every tick.
pub(crate) struct SnakeState {
    /// Snake body cells, head at the front.
    pub(crate) cells: VecDeque<IVec2>,
    /// Segment sprites, index-parallel to `cells` (0 = head, grows at tail).
    pub(crate) segments: Vec<EntityId>,
    pub(crate) direction: Direction,
    /// Turns waiting to apply, one per tick (capped at `INPUT_QUEUE_CAP`).
    pub(crate) input_queue: VecDeque<Direction>,
    pub(crate) score: u32,
    pub(crate) foods_eaten: u32,
    /// Seconds since this snake last ate (QUICK_SNACK tracking).
    pub(crate) since_last_eat: f32,
    /// Cleared the tick this snake dies; a dead snake's cells stay on screen.
    pub(crate) alive: bool,
    /// Segment tint — the theme accent for snake 0, `SNAKE2_COLOR` for snake 1.
    pub(crate) color: Vec4,
}

impl SnakeState {
    pub(crate) fn new(color: Vec4) -> Self {
        Self {
            cells: VecDeque::new(),
            segments: Vec::new(),
            direction: Direction::Right,
            input_queue: VecDeque::new(),
            score: 0,
            foods_eaten: 0,
            since_last_eat: 0.0,
            alive: true,
            color,
        }
    }
}

pub struct SnakeGame {
    /// One or two snakes sharing the grid (length depends on `mode`).
    pub(crate) snakes: Vec<SnakeState>,
    pub(crate) mode: GameMode,
    /// Food pellets shared by every snake on the board.
    pub(crate) foods: Vec<Food>,
    /// Wall frame around the playfield (dimmed in wrap-around modes).
    pub(crate) walls: Vec<EntityId>,
    pub(crate) background: Option<EntityId>,
    /// White 1x1 texture for every sprite (the whole game is neon rects).
    pub(crate) tex_id: u32,

    /// Seconds until the next grid step.
    pub(crate) tick_timer: f32,

    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Pause state + menu, driven from the `Playing` state.
    pub(crate) pause: PauseMenu,

    /// Deforming spring-mass grid drawn under the gameplay sprites.
    pub(crate) grid: Option<GridMesh>,
    /// F1 toggles magenta collider outlines over the sprites.
    pub(crate) debug_colliders: bool,
}

impl Default for SnakeGame {
    fn default() -> Self {
        Self {
            snakes: Vec::new(),
            mode: GameMode::SinglePlayer,
            foods: Vec::new(),
            walls: Vec::new(),
            background: None,
            tex_id: 0,
            tick_timer: 0.0,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            pause: PauseMenu::new(),
            grid: None,
            debug_colliders: false,
        }
    }
}
