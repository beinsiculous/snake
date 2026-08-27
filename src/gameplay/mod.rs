//! Grid movement, the step tick, food placement, and match lifecycle.
//!
//! The pure rules (`step_snake`, `resolve_versus_step`, `next_direction`,
//! `place_food`, `tick_interval`, ...) live in [`rules`] so every rule is
//! headless-testable; the `SnakeGame` methods here wire their outcomes into
//! entities, particles, and achievements.

mod rules;

pub(crate) use rules::*;

use engine_core::prelude::*;
use glam::IVec2;

use crate::constants::*;
use crate::effects;
use crate::spawning::{self, cell_to_world};
use crate::types::*;

impl SnakeGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        // F1 toggles the collider debug overlay (Snake has no colliders, so
        // it only proves the point — kept for convention parity).
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        // Pause gate: while paused the whole match is frozen — no tick, no
        // input, no timers; the overlay is drawn in the UI pass.
        if self.state == GameState::Playing {
            let action = self.pause.update(ctx.players, ctx.input, ctx.window_size);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx.world); return; }
                PauseAction::ExitGame => { ctx.exit_requested = true; return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                // Keep the frozen scene visible under the pause overlay:
                // re-emit the grid without advancing it (dt 0).
                engine_core::grid::step_and_emit_grid(
                    self.grid.as_mut(), ctx.world, ctx.lines, 0.0, self.debug_colliders,
                );
                return;
            }
        }

        self.handle_state_input(ctx);
        if self.state == GameState::Playing {
            self.buffer_direction_input(ctx);
            for snake in &mut self.snakes {
                snake.since_last_eat += ctx.delta_time;
            }

            self.tick_timer -= ctx.delta_time;
            while self.tick_timer <= 0.0 && self.state == GameState::Playing {
                self.tick_timer += tick_interval(self.chaos_mode, self.total_foods_eaten());
                self.advance(ctx);
            }
        }

        // Step + render the deforming grid after gameplay so it reacts to
        // this frame's events; collider outlines overlay when toggled.
        step_and_emit_grid(
            self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders,
        );
    }

    /// Combined foods eaten across snakes — drives the shared tick pace.
    fn total_foods_eaten(&self) -> u32 {
        self.snakes.iter().map(|s| s.foods_eaten).sum()
    }

    /// Buffer just-activated turns per player. In single player one snake
    /// listens to both players' controls; in versus each snake gets its own.
    fn buffer_direction_input(&mut self, ctx: &GameContext) {
        match self.mode {
            GameMode::SinglePlayer => {
                self.buffer_for_snake(ctx, 0, &[PlayerId::P1, PlayerId::P2])
            }
            GameMode::TwoPlayerVersus => {
                self.buffer_for_snake(ctx, 0, &[PlayerId::P1]);
                self.buffer_for_snake(ctx, 1, &[PlayerId::P2]);
            }
        }
    }

    /// Queue turns for snake `i` from any of `players`, newest last. The queue
    /// only takes a turn that differs from the one before it, so holding a
    /// direction doesn't flood the buffer.
    fn buffer_for_snake(&mut self, ctx: &GameContext, i: usize, players: &[PlayerId]) {
        const DIR_ACTIONS: [(GameAction, Direction); 4] = [
            (GameAction::MoveUp, Direction::Up),
            (GameAction::MoveDown, Direction::Down),
            (GameAction::MoveLeft, Direction::Left),
            (GameAction::MoveRight, Direction::Right),
        ];
        match self.snakes.get(i) {
            Some(snake) if snake.alive => {}
            _ => return,
        }
        for (action, dir) in DIR_ACTIONS {
            let pressed = players
                .iter()
                .any(|&p| ctx.players.just_activated(p, action, ctx.input));
            let queue = &mut self.snakes[i].input_queue;
            if pressed && queue.len() < INPUT_QUEUE_CAP && queue.back() != Some(&dir) {
                queue.push_back(dir);
            }
        }
    }

    fn advance(&mut self, ctx: &mut GameContext) {
        match self.mode {
            GameMode::SinglePlayer => self.advance_single(ctx),
            GameMode::TwoPlayerVersus => self.advance_versus(ctx),
        }
    }

    /// One single-player grid step: apply a buffered turn, move, resolve.
    fn advance_single(&mut self, ctx: &mut GameContext) {
        let dir = next_direction(self.snakes[0].direction, &mut self.snakes[0].input_queue);
        self.snakes[0].direction = dir;
        let food_cells: Vec<IVec2> = self.foods.iter().map(|f| f.cell).collect();
        let outcome = step_snake(
            &mut self.snakes[0].cells, dir, &food_cells, walls_wrap(self.chaos_mode));

        match outcome {
            StepOutcome::Moved => {}
            StepOutcome::Ate(cell) => self.eat_food(ctx, 0, cell),
            StepOutcome::Died(cause) => {
                self.finish_solo(ctx, cause);
                return;
            }
        }
        self.sync_segment_sprites(ctx.world, 0);
    }

    /// One versus grid step: both snakes resolve from the pre-step board, then
    /// only the survivors advance. The round ends the first tick anyone dies.
    fn advance_versus(&mut self, ctx: &mut GameContext) {
        for snake in self.snakes.iter_mut().take(2) {
            let dir = next_direction(snake.direction, &mut snake.input_queue);
            snake.direction = dir;
        }
        let food_cells: Vec<IVec2> = self.foods.iter().map(|f| f.cell).collect();
        let wrap = walls_wrap(self.chaos_mode);
        let steps = resolve_versus_step(
            [&self.snakes[0].cells, &self.snakes[1].cells],
            [self.snakes[0].direction, self.snakes[1].direction],
            &food_cells,
            wrap,
        );

        // Advance cells first so food that respawns during an eat avoids both
        // snakes' final positions.
        for (snake, step) in self.snakes.iter_mut().zip(steps) {
            match step {
                VersusStep::Moved(head) => {
                    snake.cells.push_front(head);
                    snake.cells.pop_back();
                }
                VersusStep::Ate(head) => snake.cells.push_front(head),
                VersusStep::Died(_) => snake.alive = false,
            }
        }
        for (i, step) in steps.iter().enumerate() {
            if let VersusStep::Ate(cell) = step {
                self.eat_food(ctx, i, *cell);
            }
        }
        for i in 0..self.snakes.len() {
            self.sync_segment_sprites(ctx.world, i);
        }

        if let Some(result) = versus_result(&steps) {
            self.finish_versus(ctx, &steps, result);
        }
    }

    /// Bookkeeping after snake `i`'s head grew onto `cell`: extend its body,
    /// replace the eaten pellet, splash particles, and score. Assumes the new
    /// head is already at the front of the snake's cells.
    fn eat_food(&mut self, ctx: &mut GameContext, i: usize, cell: IVec2) {
        self.snakes[i].score += FOOD_POINTS;
        self.snakes[i].foods_eaten += 1;

        // The snake grew: give the new tail cell its sprite in the snake's tint.
        let color = self.snakes[i].color;
        let tail = *self.snakes[i].cells.back().expect("snake grew, tail exists");
        let seg = spawning::spawn_segment(ctx.world, self.tex_id, tail, color, false);
        self.snakes[i].segments.push(seg);

        // Replace the eaten pellet with a fresh one elsewhere.
        if let Some(idx) = self.foods.iter().position(|f| f.cell == cell) {
            let food = self.foods.swap_remove(idx);
            ctx.world.remove_entity(&food.entity).ok();
        }
        self.spawn_missing_food(ctx.world);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let pos = cell_to_world(cell);
        ctx.particles.spawn_burst(pos, &effects::food_burst(&theme, self.tex_id));
        self.ripple_grid(pos, GRID_IMPULSE_EAT_STRENGTH, GRID_IMPULSE_EAT_RADIUS);

        // Achievements are a single-player pursuit; versus rounds skip them.
        if self.mode == GameMode::SinglePlayer {
            self.unlock_food_achievements(ctx);
        }
        self.snakes[i].since_last_eat = 0.0;
    }

    /// Top the board back up to the mode's pellet count, avoiding every snake.
    pub(crate) fn spawn_missing_food(&mut self, world: &mut World) {
        while self.foods.len() < food_count(self.chaos_mode) {
            let occupied: Vec<IVec2> = self.snakes.iter()
                .flat_map(|s| s.cells.iter().copied())
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

    /// Reposition snake `i`'s segment sprites onto their cells.
    fn sync_segment_sprites(&self, world: &mut World, i: usize) {
        let snake = &self.snakes[i];
        for (cell, entity) in snake.cells.iter().zip(snake.segments.iter()) {
            if let Some(t) = world.get_mut::<Transform2D>(*entity) {
                t.position = cell_to_world(*cell);
            }
        }
    }

    /// Keys that change the game state while the simulation screens are up.
    /// Either player's menu/primary action counts.
    fn handle_state_input(&mut self, ctx: &mut GameContext) {
        // Playing's Menu edge is consumed by the pause gate, not here.
        if let GameState::GameOver { .. } = &self.state {
            if ctx.players.just_activated_any(GameAction::Action1, ctx.input) {
                self.start_game(ctx);
            } else if ctx.players.just_activated_any(GameAction::Menu, ctx.input) {
                self.reset_to_title(ctx.world);
            }
        }
    }

    /// Reset and respawn the snakes for the current `mode`, deal fresh food,
    /// and begin play. Called from mode select and game-over restart.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.destroy_snakes_and_food(ctx.world);
        self.tick_timer = tick_interval(self.chaos_mode, 0);

        let accent = ChaosTheme::for_mode(self.chaos_mode).accent_color;
        match self.mode {
            GameMode::SinglePlayer => {
                let head = IVec2::new(GRID_COLS / 2, GRID_ROWS / 2);
                let snake = self.build_snake(ctx.world, head, Direction::Right, accent);
                self.snakes = vec![snake];
            }
            GameMode::TwoPlayerVersus => {
                let (h0, d0) = versus_spawn(0);
                let (h1, d1) = versus_spawn(1);
                let s0 = self.build_snake(ctx.world, h0, d0, accent);
                let s1 = self.build_snake(ctx.world, h1, d1, SNAKE2_COLOR);
                self.snakes = vec![s0, s1];
            }
        }
        self.spawn_missing_food(ctx.world);

        self.apply_theme(ctx.world);
        self.state = GameState::Playing;
    }

    /// Build a snake with its body cells and segment sprites, head glowing.
    fn build_snake(
        &self,
        world: &mut World,
        head: IVec2,
        direction: Direction,
        color: Vec4,
    ) -> SnakeState {
        let mut snake = SnakeState::new(color);
        snake.direction = direction;
        for (i, cell) in starting_body(head, direction, START_LENGTH).into_iter().enumerate() {
            snake.cells.push_back(cell);
            snake.segments.push(spawning::spawn_segment(
                world, self.tex_id, cell, color, i == 0));
        }
        snake
    }

    /// End a single-player run. Entities stay on screen behind the overlay;
    /// the next start rebuilds them.
    fn finish_solo(&mut self, ctx: &mut GameContext, cause: DeathCause) {
        self.snakes[0].alive = false;
        let head_pos = cell_to_world(*self.snakes[0].cells.front().expect("snake has a head"));
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(head_pos, &effects::death_burst(&theme, self.tex_id));
        self.ripple_grid(head_pos, GRID_IMPULSE_DEATH_STRENGTH, GRID_IMPULSE_DEATH_RADIUS);

        self.unlock_death_achievements(ctx, cause);
        ctx.scores.submit("solo", self.snakes[0].score as u64);
        self.state = GameState::GameOver { result: GameResult::Solo(cause) };
    }

    /// End a versus round: shatter each dead snake's head, then show the result.
    fn finish_versus(&mut self, ctx: &mut GameContext, steps: &[VersusStep; 2], result: GameResult) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let dead_heads: Vec<Vec2> = steps.iter().zip(self.snakes.iter())
            .filter(|(step, _)| matches!(step, VersusStep::Died(_)))
            .filter_map(|(_, snake)| snake.cells.front().copied())
            .map(cell_to_world)
            .collect();
        for pos in dead_heads {
            ctx.particles.spawn_burst(pos, &effects::death_burst(&theme, self.tex_id));
            self.ripple_grid(pos, GRID_IMPULSE_DEATH_STRENGTH, GRID_IMPULSE_DEATH_RADIUS);
        }
        for snake in &self.snakes {
            ctx.scores.submit("versus", snake.score as u64);
        }
        self.state = GameState::GameOver { result };
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_snakes_and_food(world);
        self.state = GameState::TitleScreen { selection: 0 };
    }

    fn destroy_snakes_and_food(&mut self, world: &mut World) {
        for snake in &mut self.snakes {
            for entity in snake.segments.drain(..) {
                world.remove_entity(&entity).ok();
            }
        }
        self.snakes.clear();
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
        let entities: Vec<EntityId> = self.snakes.iter()
            .flat_map(|s| s.segments.iter().copied())
            .chain(self.foods.iter().map(|f| f.entity))
            .chain(self.walls.iter().copied())
            .collect();
        set_sprites_visible(ctx.world, entities, visible);
    }
}
