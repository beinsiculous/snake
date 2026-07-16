use engine_core::prelude::*;

pub(crate) const WIN_W: f32 = 800.0;
pub(crate) const WIN_H: f32 = 600.0;

// --- Playfield grid ---
pub(crate) const GRID_COLS: i32 = 26;
pub(crate) const GRID_ROWS: i32 = 16;
pub(crate) const CELL_PX: f32 = 28.0;
/// The playfield sits slightly below window center to leave a HUD band on top.
pub(crate) const PLAYFIELD_OFFSET_Y: f32 = -10.0;
/// Thickness of the wall frame drawn around the playfield.
pub(crate) const WALL_THICKNESS: f32 = 6.0;

// --- Snake ---
pub(crate) const START_LENGTH: usize = 3;
/// Buffered turns waiting to be applied (classic two-turns-ahead feel).
pub(crate) const INPUT_QUEUE_CAP: usize = 2;
/// Sprites are drawn slightly smaller than the cell so segments read as
/// individual links instead of one solid bar.
pub(crate) const SEGMENT_PX: f32 = CELL_PX - 4.0;
pub(crate) const FOOD_PX: f32 = CELL_PX - 10.0;

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

// --- Look ---
pub(crate) const FOOD_COLOR: Vec4 = Vec4::new(1.0, 0.35, 0.55, 1.0);
/// Player 2's snake tint in versus (player 1 wears the chaos-mode accent).
pub(crate) const SNAKE2_COLOR: Vec4 = Vec4::new(0.30, 0.90, 1.0, 1.0);
pub(crate) const HEAD_EMISSIVE: f32 = 1.6;
pub(crate) const BODY_EMISSIVE: f32 = 0.9;
pub(crate) const FOOD_EMISSIVE: f32 = 2.2;
pub(crate) const WALL_EMISSIVE: f32 = 0.7;
/// Wrap-around modes dim the walls — they're portals, not hazards.
pub(crate) const WALL_WRAP_ALPHA: f32 = 0.25;

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
