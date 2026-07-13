use std::collections::VecDeque;

use engine_core::prelude::*;
use glam::IVec2;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    ModeSelect { selection: u8 },
    Achievements,
    Playing,
    GameOver { cause: DeathCause },
}

/// What killed the snake — the game-over overlay names the culprit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeathCause {
    Wall,
    SelfBite,
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

pub(crate) struct SnakeGame {
    /// Snake body cells, head at the front. The single source of truth —
    /// segment sprites are repositioned from this every tick.
    pub(crate) cells: VecDeque<IVec2>,
    /// Segment sprites, index-parallel to `cells` (0 = head, grows at tail).
    pub(crate) segments: Vec<EntityId>,
    pub(crate) direction: Direction,
    /// Turns waiting to apply, one per tick (capped at `INPUT_QUEUE_CAP`).
    pub(crate) input_queue: VecDeque<Direction>,
    pub(crate) foods: Vec<Food>,
    /// Wall frame around the playfield (dimmed in wrap-around modes).
    pub(crate) walls: Vec<EntityId>,
    pub(crate) background: Option<EntityId>,
    /// White 1x1 texture for every sprite (the whole game is neon rects).
    pub(crate) tex_id: u32,

    /// Seconds until the next grid step.
    pub(crate) tick_timer: f32,
    pub(crate) score: u32,
    pub(crate) foods_eaten: u32,
    /// Seconds since the last food was eaten (QUICK_SNACK tracking).
    pub(crate) since_last_eat: f32,

    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Deforming spring-mass grid drawn under the gameplay sprites.
    pub(crate) grid: Option<GridMesh>,
    /// F1 toggles magenta collider outlines over the sprites.
    pub(crate) debug_colliders: bool,
}

impl Default for SnakeGame {
    fn default() -> Self {
        Self {
            cells: VecDeque::new(),
            segments: Vec::new(),
            direction: Direction::Right,
            input_queue: VecDeque::new(),
            foods: Vec::new(),
            walls: Vec::new(),
            background: None,
            tex_id: 0,
            tick_timer: 0.0,
            score: 0,
            foods_eaten: 0,
            since_last_eat: 0.0,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            grid: None,
            debug_colliders: false,
        }
    }
}
