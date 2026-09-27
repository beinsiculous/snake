use std::collections::VecDeque;

use engine_core::prelude::*;
use glam::IVec2;

use crate::gameplay::Snack;

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

/// Which dog a snake is: player one's Frank, or player two's recoloured Frank. The
/// choice picks the sheet, the draw depth and the death burst's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dog {
    Frank,
    PlayerTwo,
}

impl Dog {
    /// What the editor hierarchy calls this dog's body.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Dog::Frank => "Frank",
            Dog::PlayerTwo => "Frank (player 2)",
        }
    }
}

/// A live food pellet: its grid cell, which snack it is, and the sprite showing it.
pub(crate) struct Food {
    pub(crate) cell: IVec2,
    pub(crate) snack: Snack,
    pub(crate) entity: EntityId,
}

/// One snake's complete state. Single-player uses `snakes[0]`; versus adds a
/// second. `cells` is the source of truth — the dog's `Tilemap` is rewritten from it
/// every frame.
pub(crate) struct SnakeState {
    /// Snake body cells, head at the front.
    pub(crate) cells: VecDeque<IVec2>,
    /// The entity carrying this dog's body `Tilemap`.
    pub(crate) map: EntityId,
    pub(crate) dog: Dog,
    pub(crate) direction: Direction,
    /// Turns waiting to apply, one per tick (capped at `INPUT_QUEUE_CAP`).
    pub(crate) input_queue: VecDeque<Direction>,
    pub(crate) score: u32,
    pub(crate) foods_eaten: u32,
    /// Seconds since this snake last ate (QUICK_SNACK tracking).
    pub(crate) since_last_eat: f32,
    /// Cleared the tick this snake dies; a dead snake's cells stay on screen.
    pub(crate) alive: bool,
    /// The match's play time when this dog died: its hurt clip plays from here.
    pub(crate) death_time: Option<f32>,
}

impl SnakeState {
    pub(crate) fn new(dog: Dog, map: EntityId) -> Self {
        Self {
            cells: VecDeque::new(),
            map,
            dog,
            direction: Direction::Right,
            input_queue: VecDeque::new(),
            score: 0,
            foods_eaten: 0,
            since_last_eat: 0.0,
            alive: true,
            death_time: None,
        }
    }
}

/// An eaten snack's collect, still playing on the cell it was eaten from.
pub(crate) struct Collect {
    pub(crate) entity: EntityId,
    pub(crate) cell: IVec2,
}

/// Every loaded sheet, plus the white texture the particles use.
pub(crate) struct Sheets {
    pub(crate) white: u32,
    pub(crate) frank: SpriteSheet,
    pub(crate) frank_player_two: SpriteSheet,
    pub(crate) pretzel: SpriteSheet,
    pub(crate) cheese_bite: SpriteSheet,
    pub(crate) bacon_bone: SpriteSheet,
    pub(crate) kitchen_floor: SpriteSheet,
    pub(crate) kitchen_wall: SpriteSheet,
}

impl Sheets {
    pub(crate) fn dog(&self, dog: Dog) -> &SpriteSheet {
        match dog {
            Dog::Frank => &self.frank,
            Dog::PlayerTwo => &self.frank_player_two,
        }
    }

    pub(crate) fn snack(&self, snack: Snack) -> &SpriteSheet {
        match snack {
            Snack::Pretzel => &self.pretzel,
            Snack::CheeseBite => &self.cheese_bite,
            Snack::BaconBone => &self.bacon_bone,
        }
    }
}

/// A sheet with no texture, one cell and no clips. `Sheets::default` holds these until
/// `init()` loads the real ones: the engine builds the game with `Default` and calls
/// `init` on the first frame, before any entity that could draw exists.
fn placeholder_sheet() -> SpriteSheet {
    SpriteSheet {
        texture: TextureHandle { id: 0 },
        grid: SheetGrid::new(1, 1),
        clips: Vec::new(),
        path: String::new(),
    }
}

impl Default for Sheets {
    fn default() -> Self {
        Self {
            white: 0,
            frank: placeholder_sheet(),
            frank_player_two: placeholder_sheet(),
            pretzel: placeholder_sheet(),
            cheese_bite: placeholder_sheet(),
            bacon_bone: placeholder_sheet(),
            kitchen_floor: placeholder_sheet(),
            kitchen_wall: placeholder_sheet(),
        }
    }
}

pub struct SnakeGame {
    /// One or two snakes sharing the grid (length depends on `mode`).
    pub(crate) snakes: Vec<SnakeState>,
    pub(crate) mode: GameMode,
    /// Food pellets shared by every snake on the board.
    pub(crate) foods: Vec<Food>,
    pub(crate) sheets: Sheets,
    /// The kitchen floor's `Tilemap`, under everything.
    pub(crate) floor: Option<EntityId>,
    /// The kitchen wall's `Tilemap`: the ring around the floor, open in the wrap modes.
    pub(crate) wall: Option<EntityId>,
    /// The deforming grid drawn over the floor; the engine steps and draws it.
    pub(crate) backdrop: Option<EntityId>,
    /// Collects still playing: a restart or a quit ends them, and a fresh pellet never
    /// lands on one's cell.
    pub(crate) collects: Vec<Collect>,

    /// Seconds until the next grid step.
    pub(crate) tick_timer: f32,
    /// Seconds of match played, frozen only by the pause: it drives the wag and the hurt.
    pub(crate) play_time: f32,

    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Pause state + menu, driven from the `Playing` state.
    pub(crate) pause: PauseMenu,

    /// F1 toggles magenta collider outlines over the sprites.
    pub(crate) debug_colliders: bool,
}

impl Default for SnakeGame {
    fn default() -> Self {
        Self {
            snakes: Vec::new(),
            mode: GameMode::SinglePlayer,
            foods: Vec::new(),
            sheets: Sheets::default(),
            floor: None,
            wall: None,
            backdrop: None,
            collects: Vec::new(),
            tick_timer: 0.0,
            play_time: 0.0,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            pause: PauseMenu::new(),
            debug_colliders: false,
        }
    }
}
