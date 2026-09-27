//! Grid movement, the step tick, food placement, and match lifecycle.
//!
//! The pure rules (`step_snake`, `resolve_versus_step`, `next_direction`,
//! `place_food`, `tick_interval`, `piece_for`, `snack_for`, ...) live in [`rules`] so
//! every rule is headless-testable; the `SnakeGame` methods here wire their outcomes
//! into entities, particles, and achievements.

mod rules;

pub(crate) use rules::*;

use engine_core::prelude::*;
use glam::IVec2;

use crate::body::retile_dog;
use crate::constants::*;
use crate::effects;
use crate::spawning::{self, cell_to_world};
use crate::types::*;

/// Push a radial shockwave into the backdrop grid. The engine applies it to every
/// backdrop on its next running frame.
fn ripple_grid(world: &mut World, position: Vec2, strength: f32, radius: f32) {
    ripple(world, GridImpulse::Radial { position, strength, radius, attractive: false });
}

/// The backdrop grid's colour for a chaos theme: its grid colour at the low alpha that
/// lets the lattice read over the floor without veiling it.
pub(crate) fn backdrop_color(theme: &ChaosTheme) -> Vec4 {
    let grid = theme.grid_color;
    Vec4::new(grid.x, grid.y, grid.z, BACKDROP_ALPHA)
}

impl SnakeGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        // F1 toggles the collider debug overlay (Snake has no colliders, so
        // it only proves the point — kept for convention parity).
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        // Pause gate: while paused the whole match is frozen — no tick, no
        // input, no timers, no wag; the engine holds the backdrop still, and the
        // overlay is drawn in the UI pass.
        if self.state == GameState::Playing {
            let action = self.pause.update(ctx.players, ctx.input, ctx.window_size);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx.world); return; }
                PauseAction::ExitGame => { ctx.request_exit(); return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                return;
            }
        }

        // The clock runs through game over too: a dead dog's hurt plays out behind the
        // panel.
        self.play_time += ctx.delta_time;

        self.handle_state_input(ctx);
        if self.state == GameState::Playing {
            self.buffer_direction_input(ctx);
            for snake in &mut self.snakes {
                snake.since_last_eat += ctx.delta_time;
            }

            // A collect covering the last free cell holds a pellet back; once it has
            // played out, the pellet is placed without waiting for another eat.
            if self.foods.len() < food_count(self.chaos_mode) {
                self.forget_finished_collects(ctx.world);
                self.spawn_missing_food(ctx.world);
            }

            self.tick_timer -= ctx.delta_time;
            while self.tick_timer <= 0.0 && self.state == GameState::Playing {
                self.tick_timer += tick_interval(self.chaos_mode, self.total_foods_eaten());
                self.advance(ctx);
            }
        }

        // After every tick of the frame: the dogs' maps have one writer.
        self.retile_dogs(ctx.world);

        if self.debug_colliders {
            debug::draw_colliders(ctx.world, ctx.lines, DEBUG_COLLIDER_COLOR, DEBUG_COLLIDER_EMISSIVE);
        }
    }

    /// Rewrite every dog's body map from its cells and the play clock.
    pub(crate) fn retile_dogs(&self, world: &mut World) {
        for snake in &self.snakes {
            let sheet = self.sheets.dog(snake.dog);
            if let Some(map) = world.get_mut::<Tilemap>(snake.map) {
                retile_dog(map, sheet, &snake.cells, self.play_time, snake.death_time);
            }
        }
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
            StepOutcome::Died(cause) => self.finish_solo(ctx, cause),
        }
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

        if let Some(result) = versus_result(&steps) {
            self.finish_versus(ctx, &steps, result);
        }
    }

    /// Bookkeeping after snake `i`'s head grew onto `cell`: score, replace the eaten
    /// snack with its collect and a fresh pellet elsewhere, crumbs and a ripple. The
    /// body itself needs nothing: the map is rewritten from `cells` after the tick.
    fn eat_food(&mut self, ctx: &mut GameContext, i: usize, cell: IVec2) {
        self.snakes[i].score += FOOD_POINTS;
        self.snakes[i].foods_eaten += 1;

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let pos = cell_to_world(cell);
        if let Some(index) = self.foods.iter().position(|f| f.cell == cell) {
            let food = self.foods.swap_remove(index);
            ctx.world.remove_entity(&food.entity).ok();
            self.forget_finished_collects(ctx.world);
            let sheet = self.sheets.snack(food.snack);
            let entity = spawning::spawn_collect(ctx.world, sheet, food.snack, cell);
            self.collects.push(Collect { entity, cell });
            let color = snack_art(food.snack).burst_color;
            ctx.particles.spawn_burst(pos, &effects::food_burst(&theme, self.sheets.white, color));
        }
        self.spawn_missing_food(ctx.world);
        ripple_grid(ctx.world, pos, GRID_IMPULSE_EAT_STRENGTH, GRID_IMPULSE_EAT_RADIUS);

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
                .chain(self.collects.iter().map(|collect| collect.cell))
                .collect();
            let seed = self.frame_count.wrapping_add(self.foods.len() as u32);
            let Some(cell) = place_food(&occupied, seed) else {
                return; // board full — the player has effectively won Snake
            };
            let snack = snack_for(seed);
            let entity = spawning::spawn_snack(world, self.sheets.snack(snack), snack, cell);
            self.foods.push(Food { cell, snack, entity });
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
        self.play_time = 0.0;

        match self.mode {
            GameMode::SinglePlayer => {
                let head = IVec2::new(GRID_COLS / 2, GRID_ROWS / 2);
                let snake = self.build_snake(ctx.world, head, Direction::Right, Dog::Frank);
                self.snakes = vec![snake];
            }
            GameMode::TwoPlayerVersus => {
                let (h0, d0) = versus_spawn(0);
                let (h1, d1) = versus_spawn(1);
                let s0 = self.build_snake(ctx.world, h0, d0, Dog::Frank);
                let s1 = self.build_snake(ctx.world, h1, d1, Dog::PlayerTwo);
                self.snakes = vec![s0, s1];
            }
        }
        self.spawn_missing_food(ctx.world);

        self.apply_theme(ctx.world);
        self.retile_dogs(ctx.world);
        self.state = GameState::Playing;
    }

    /// Build a snake: its starting cells and its body map.
    fn build_snake(&self, world: &mut World, head: IVec2, direction: Direction, dog: Dog) -> SnakeState {
        let map = spawning::spawn_dog_map(world, self.sheets.dog(dog), dog);
        let mut snake = SnakeState::new(dog, map);
        snake.direction = direction;
        snake.cells.extend(starting_body(head, direction, START_LENGTH));
        snake
    }

    /// Mark snake `i` dead now: its hurt plays from this moment, and its head bursts.
    fn kill(&mut self, ctx: &mut GameContext, i: usize) {
        let snake = &mut self.snakes[i];
        snake.alive = false;
        snake.death_time = Some(self.play_time);
        let head_pos = cell_to_world(*snake.cells.front().expect("snake has a head"));
        let color = dog_art(snake.dog).burst_color;
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(head_pos, &effects::death_burst(&theme, self.sheets.white, color));
        ripple_grid(ctx.world, head_pos, GRID_IMPULSE_DEATH_STRENGTH, GRID_IMPULSE_DEATH_RADIUS);
    }

    /// End a single-player run. Entities stay on screen behind the overlay;
    /// the next start rebuilds them.
    fn finish_solo(&mut self, ctx: &mut GameContext, cause: DeathCause) {
        self.kill(ctx, 0);
        self.unlock_death_achievements(ctx, cause);
        let _ = ctx.scores.submit("solo", self.snakes[0].score as u64);
        self.state = GameState::GameOver { result: GameResult::Solo(cause) };
    }

    /// End a versus round: every dog that died this tick shows it, then the result.
    fn finish_versus(&mut self, ctx: &mut GameContext, steps: &[VersusStep; 2], result: GameResult) {
        for (i, step) in steps.iter().enumerate() {
            if matches!(step, VersusStep::Died(_)) {
                self.kill(ctx, i);
            }
        }
        for snake in &self.snakes {
            let _ = ctx.scores.submit("versus", snake.score as u64);
        }
        self.state = GameState::GameOver { result };
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_snakes_and_food(world);
        self.apply_theme(world);
        self.state = GameState::TitleScreen { selection: 0 };
    }

    /// Everything a match owns: the dogs' maps, the pellets and any collect still
    /// playing — a stale collect id is skipped, never a newer entity's. A map cannot be
    /// hidden, so a dog's is despawned with its match.
    fn destroy_snakes_and_food(&mut self, world: &mut World) {
        for snake in self.snakes.drain(..) {
            world.remove_entity(&snake.map).ok();
        }
        for food in self.foods.drain(..) {
            world.remove_entity(&food.entity).ok();
        }
        for collect in self.collects.drain(..) {
            world.remove_entity(&collect.entity).ok();
        }
    }

    /// Forget the collects that have played out and despawned themselves, so the list
    /// holds only the ones still on screen. An id is generational: a despawned collect's
    /// never matches a newer entity.
    fn forget_finished_collects(&mut self, world: &World) {
        self.collects.retain(|collect| world.get::<Sprite>(collect.entity).is_some());
    }

    /// Push the current `chaos_mode`'s look onto the live entities: the backdrop grid's
    /// colour, and the wall's ring — open in the wrap modes, where the dog leaves by one
    /// side and comes back by the other.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(grid) = self.backdrop.and_then(|backdrop| world.get_mut::<GridBackdrop>(backdrop)) {
            grid.color = backdrop_color(&theme);
        }
        if let Some(map) = self.wall.and_then(|wall| world.get_mut::<Tilemap>(wall)) {
            spawning::write_ring(map, &self.sheets.kitchen_wall, walls_wrap(self.chaos_mode));
        }
    }

    /// Snacks only exist on screen outside the menu screens; the floor, the ring and
    /// the grid stay up under the menus.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = matches!(self.state, GameState::Playing | GameState::GameOver { .. });
        let entities: Vec<EntityId> = self.foods.iter().map(|f| f.entity)
            .chain(self.collects.iter().map(|collect| collect.entity))
            .collect();
        set_sprites_visible(ctx.world, entities, visible);
    }
}
