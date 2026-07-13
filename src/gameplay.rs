//! Grid movement, the step tick, food placement, and match lifecycle.
//!
//! The core rules (`step_snake`, `next_direction`, `place_food`,
//! `tick_interval`) are pure functions over plain data so every rule is
//! headless-testable; the `SnakeGame` methods wire their outcomes into
//! entities, particles, and achievements.

use std::collections::VecDeque;

use engine_core::prelude::*;
use glam::IVec2;

use crate::constants::*;
use crate::effects;
use crate::spawning::{self, cell_to_world, in_bounds};
use crate::types::*;

/// What one grid step did. The caller turns this into entities/effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepOutcome {
    Moved,
    /// The head landed on the food at this cell; the snake grew by one.
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
    let head = *cells.front().expect("snake always has a head");
    let mut new_head = head + direction.delta();

    if wrap {
        new_head.x = new_head.x.rem_euclid(GRID_COLS);
        new_head.y = new_head.y.rem_euclid(GRID_ROWS);
    } else if !in_bounds(new_head) {
        return StepOutcome::Died(DeathCause::Wall);
    }

    let eating = foods.contains(&new_head);
    // The tail cell vacates this step unless we grow, so stepping into it is
    // legal — exclude it from the self-bite check.
    let body_len = if eating { cells.len() } else { cells.len() - 1 };
    if cells.iter().take(body_len).any(|&c| c == new_head) {
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

impl SnakeGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        // F1 toggles the collider debug overlay (Snake has no colliders, so
        // it only proves the point — kept for convention parity).
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        self.handle_state_input(ctx);
        if self.state == GameState::Playing {
            self.buffer_direction_input(ctx);
            self.since_last_eat += ctx.delta_time;

            self.tick_timer -= ctx.delta_time;
            while self.tick_timer <= 0.0 && self.state == GameState::Playing {
                self.tick_timer += tick_interval(self.chaos_mode, self.foods_eaten);
                self.advance_snake(ctx);
            }
        }

        // Step + render the deforming grid after gameplay so it reacts to
        // this frame's events; collider outlines overlay when toggled.
        step_and_emit_grid(
            self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders,
        );
    }

    /// Queue just-pressed turns (WASD/arrows), newest last. The queue only
    /// takes a turn that differs from the one before it, so holding a key
    /// doesn't flood the buffer.
    fn buffer_direction_input(&mut self, ctx: &GameContext) {
        let presses = [
            (KeyCode::ArrowUp, Direction::Up), (KeyCode::KeyW, Direction::Up),
            (KeyCode::ArrowDown, Direction::Down), (KeyCode::KeyS, Direction::Down),
            (KeyCode::ArrowLeft, Direction::Left), (KeyCode::KeyA, Direction::Left),
            (KeyCode::ArrowRight, Direction::Right), (KeyCode::KeyD, Direction::Right),
        ];
        for (key, dir) in presses {
            if ctx.input.is_key_just_pressed(key)
                && self.input_queue.len() < INPUT_QUEUE_CAP
                && self.input_queue.back() != Some(&dir)
            {
                self.input_queue.push_back(dir);
            }
        }
    }

    /// One grid step: apply a buffered turn, move, then resolve the outcome.
    fn advance_snake(&mut self, ctx: &mut GameContext) {
        self.direction = next_direction(self.direction, &mut self.input_queue);
        let food_cells: Vec<IVec2> = self.foods.iter().map(|f| f.cell).collect();
        let outcome = step_snake(
            &mut self.cells, self.direction, &food_cells, walls_wrap(self.chaos_mode));

        match outcome {
            StepOutcome::Moved => {}
            StepOutcome::Ate(cell) => self.eat_food(ctx, cell),
            StepOutcome::Died(cause) => {
                self.finish_game(ctx, cause);
                return;
            }
        }
        self.sync_segment_sprites(ctx.world);
    }

    fn eat_food(&mut self, ctx: &mut GameContext, cell: IVec2) {
        self.score += FOOD_POINTS;
        self.foods_eaten += 1;

        // The snake grew: give the new tail cell its sprite.
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let tail = *self.cells.back().expect("snake grew, tail exists");
        self.segments.push(spawning::spawn_segment(
            ctx.world, self.tex_id, tail, theme.accent_color, false));

        // Replace the eaten pellet with a fresh one elsewhere.
        if let Some(i) = self.foods.iter().position(|f| f.cell == cell) {
            let food = self.foods.swap_remove(i);
            ctx.world.remove_entity(&food.entity).ok();
        }
        self.spawn_missing_food(ctx.world);

        let pos = cell_to_world(cell);
        ctx.particles.spawn_burst(pos, &effects::food_burst(&theme, self.tex_id));
        self.ripple_grid(pos, GRID_IMPULSE_EAT_STRENGTH, GRID_IMPULSE_EAT_RADIUS);

        self.unlock_food_achievements(ctx);
        self.since_last_eat = 0.0;
    }

    /// Top the board back up to the mode's pellet count.
    pub(crate) fn spawn_missing_food(&mut self, world: &mut World) {
        while self.foods.len() < food_count(self.chaos_mode) {
            let occupied: Vec<IVec2> = self.cells.iter().copied()
                .chain(self.foods.iter().map(|f| f.cell))
                .collect();
            let salt = self.foods.len() as u32;
            let Some(cell) = place_food(&occupied, self.frame_count.wrapping_add(salt)) else {
                return; // board full — the player has effectively won Snake
            };
            let entity = spawning::spawn_food_sprite(world, self.tex_id, cell);
            self.foods.push(Food { cell, entity });
        }
    }

    /// Reposition every segment sprite onto its cell.
    fn sync_segment_sprites(&self, world: &mut World) {
        for (cell, entity) in self.cells.iter().zip(self.segments.iter()) {
            if let Some(t) = world.get_mut::<Transform2D>(*entity) {
                t.position = cell_to_world(*cell);
            }
        }
    }

    /// Keys that change the game state while the simulation screens are up.
    fn handle_state_input(&mut self, ctx: &mut GameContext) {
        match &self.state {
            GameState::Playing => {
                if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                }
            }
            GameState::GameOver { .. } => {
                if ctx.input.is_key_just_pressed(KeyCode::Space)
                    || ctx.input.is_key_just_pressed(KeyCode::Enter)
                {
                    self.start_game(ctx);
                } else if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                }
            }
            _ => {}
        }
    }

    /// Reset score and body, respawn the snake mid-field heading right, and
    /// deal fresh food. Called from mode select and game-over restart.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.destroy_snake_and_food(ctx.world);
        self.score = 0;
        self.foods_eaten = 0;
        self.since_last_eat = 0.0;
        self.direction = Direction::Right;
        self.input_queue.clear();
        self.tick_timer = tick_interval(self.chaos_mode, 0);

        // Head mid-field, body trailing left, moving right.
        let head = IVec2::new(GRID_COLS / 2, GRID_ROWS / 2);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        for i in 0..START_LENGTH as i32 {
            let cell = IVec2::new(head.x - i, head.y);
            self.cells.push_back(cell);
            self.segments.push(spawning::spawn_segment(
                ctx.world, self.tex_id, cell, theme.accent_color, i == 0));
        }
        self.spawn_missing_food(ctx.world);

        self.apply_theme(ctx.world);
        self.state = GameState::Playing;
    }

    /// End the run. Entities stay on screen behind the game-over overlay;
    /// the next start rebuilds them.
    fn finish_game(&mut self, ctx: &mut GameContext, cause: DeathCause) {
        let head_pos = cell_to_world(*self.cells.front().expect("snake has a head"));
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(head_pos, &effects::death_burst(&theme, self.tex_id));
        self.ripple_grid(head_pos, GRID_IMPULSE_DEATH_STRENGTH, GRID_IMPULSE_DEATH_RADIUS);

        self.unlock_death_achievements(ctx, cause);
        self.state = GameState::GameOver { cause };
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_snake_and_food(world);
        self.state = GameState::TitleScreen { selection: 0 };
    }

    fn destroy_snake_and_food(&mut self, world: &mut World) {
        for entity in self.segments.drain(..) {
            world.remove_entity(&entity).ok();
        }
        self.cells.clear();
        for food in self.foods.drain(..) {
            world.remove_entity(&food.entity).ok();
        }
    }

    /// Push the current `chaos_mode`'s look onto the live entities:
    /// background tint, wall color, and a fresh grid.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(bg) = self.background {
            if let Some(s) = world.get_mut::<Sprite>(bg) { s.color = theme.bg_color; }
        }
        // Wrap-around modes dim the walls — they're portals, not hazards.
        let mut wall_color = theme.structure_color;
        if walls_wrap(self.chaos_mode) {
            wall_color.w *= WALL_WRAP_ALPHA;
        }
        for &wall in &self.walls {
            if let Some(s) = world.get_mut::<Sprite>(wall) { s.color = wall_color; }
        }
        self.grid = Some(default_playfield_grid(&theme));
    }

    /// Push a radial shockwave into the deforming grid.
    fn ripple_grid(&mut self, position: Vec2, strength: f32, radius: f32) {
        if let Some(grid) = self.grid.as_mut() {
            grid.apply_impulse(&GridImpulse::Radial { position, strength, radius, attractive: false });
        }
    }

    /// Gameplay sprites only exist on screen outside the menu screens.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = matches!(self.state, GameState::Playing | GameState::GameOver { .. });
        let entities: Vec<EntityId> = self.segments.iter().copied()
            .chain(self.foods.iter().map(|f| f.entity))
            .chain(self.walls.iter().copied())
            .collect();
        set_sprites_visible(ctx.world, entities, visible);
    }
}
