# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. (`AGENTS.md` is a symlink to this file.)

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # run all 38 tests (headless, finish in well under a second)
cargo test <test_name>        # run a single test
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` feature). The `deion_assets` symlink at the repo root (`-> ../deion_assets`) makes the same assumption.

## Architecture

This is a single-crate game (`insiculous_snake`) built on the in-house `insiculous_2d` ECS engine. `SnakeGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs` — `init()` spawns the background/walls and registers achievements, `update()` runs once per frame. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`; no game code changes between the two modes.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()` in main.rs: `TitleScreen` / `ModeSelect` / `Achievements` dispatch to handlers in `menu.rs`; everything else (`Playing`, `GameOver`) falls through to `update_gameplay()` in `gameplay/mod.rs`. Flow is Title (1 Player / 2 Player Versus / Achievements / Exit) → ModeSelect (which is the **chaos-mode** select — the player-count choice already happened on the title) → Playing ↔ GameOver. Match lifecycle (`start_game`, `reset_to_title`) lives in `gameplay/mod.rs`; every frame ends with `update_entity_visibility()` (gameplay sprites are hidden on menu screens) and `draw_ui()` (drawing.rs).

**Pure rules / entity wiring split.** All grid rules are plain-data functions in `src/gameplay/rules.rs` — `next_direction`, `step_snake`, `resolve_versus_step`, `versus_result`, `place_food`, `tick_interval`, `food_count`, `walls_wrap`, `versus_spawn`, `starting_body`. No entities, no `GameContext`, fully headless-testable. The `SnakeGame` methods in `gameplay/mod.rs` turn their outcomes into entities, particles, grid ripples, and achievements. Keep new rules on the pure side of this line.

**Tick-based movement, not per-frame.** `tick_timer` counts down by `ctx.delta_time`; a `while tick_timer <= 0.0` loop calls `advance()` once per elapsed tick, so a slow frame can run multiple grid steps. The interval comes from `tick_interval(chaos_mode, total_foods_eaten())` — total across *both* snakes in versus, so either player's eating speeds the shared clock in Insane-family modes.

**The body is data, sprites follow.** `SnakeState.cells` (a `VecDeque<IVec2>`, head at the front) is the source of truth; `segments` is an index-parallel `Vec<EntityId>` of plain sprites. Movement = `push_front` new head + `pop_back` tail; eating skips the pop and spawns one new tail sprite. `sync_segment_sprites` repositions every segment's `Transform2D` from its cell each tick. All collision is grid-cell math — there is no physics anywhere in this game.

**Input buffering (the classic-feel subtlety).** Turns are queued per snake (`input_queue`, capped at `INPUT_QUEUE_CAP = 2`, no consecutive duplicates), one applied per tick. `next_direction` discards turns equal or opposite to the *current* heading and validates at apply time, so two buffered turns can never combine into a 180 reversal. In single player one snake listens to both players' controls (`PlayerId::P1` **and** `P2` — WASD, arrows, and either gamepad all steer); in versus each snake gets its own player slot. Gameplay reads only `ctx.players` `GameAction`s (Move*, Action1, Menu) — never raw key codes (F1 debug toggle excepted). Bindings persist to `saves/input_settings.json`.

**Versus resolution is simultaneous.** `resolve_versus_step` computes both snakes' steps from the *pre-step* board, so resolution is order-independent: both heads on the same cell = mutual `HeadOn`; adjacent heads swapping through each other both die (`OtherSnake` — each old head cell is still the other's body); a snake's vacating tail cell is safe to enter unless that snake is eating. Only survivors advance. First death ends the round (`versus_result`): lone survivor wins, simultaneous deaths draw. Dead snakes' bodies stay on screen behind the game-over panel. Achievements are single-player only — versus rounds never unlock.

**Food placement is deterministic.** `place_food(occupied, seed)` hashes the seed to a cell and linear-probes to the first free one; seeds come from `frame_count` (+ a salt per pellet). Returns `None` only on a full board (`spawn_missing_food` treats that as "player has won Snake" and stops). Food respawn during a versus eat happens *after* both snakes advance, so a fresh pellet avoids both final positions.

**Chaos modes** (engine `ChaosMode`, meaning defined here — see `mode_hint` in menu.rs): **Insane** = faster base tick that shrinks by `INSANE_TICK_STEP` per food eaten down to `INSANE_TICK_MIN`; **Ridiculous** = wrap-around walls (edges teleport; the wall frame dims to `WALL_WRAP_ALPHA` — portals, not hazards) + two pellets on the board; **Insiculous** = both (`is_insane()`/`is_ridiculous()` both fire). `ChaosTheme::for_mode` supplies the palette; `apply_theme` pushes it onto live entities on each `start_game`. The runtime selection is mirrored into `ctx.chaos_mode` (read-write, engine persists it).

**Pause** follows the engine's universal pattern (`PauseMenu` gate at the top of `update_gameplay`, pausable in `Playing` only): Resume skips the rest of the frame so the keypress can't leak; while paused the frozen deforming grid is re-emitted with dt 0 so the backdrop stays visible under the overlay.

**Visuals:** the whole game is neon rects off one white 1×1 texture (`tex_id`) — head glows brighter than body (`HEAD_EMISSIVE`/`BODY_EMISSIVE`), food brightest. Particle presets live in `effects.rs` (food pop, death shatter); eat/death events also kick radial impulses into the engine's spring-mass background grid. Menus use `MenuPanel`/`MenuStyle` chrome. Player 2's snake wears `SNAKE2_COLOR`; player 1 wears the chaos theme accent.

**Editor naming:** every spawned entity gets a `Name` component ("Head", "Segment ...", "Food", "Wall Top") so the editor hierarchy reads well — keep this for new entities.

**All tuning lives in `src/constants.rs`** (grid dims, tick times, colors, emissives, achievement thresholds) and all entity creation in `src/spawning.rs`. Values tuned live in the editor inspector must be copied back into constants.rs to persist.

**Paths:** assets and saves anchor to `engine_core::game_root!()` (main.rs), so `cargo run` works from any cwd. Achievements persist to `saves/snake_achievements.json`; the 9 definitions, grouped `DISPLAY_SECTIONS`, and unlock logic live in `achievements.rs` (length milestones + per-mode feasts unlock live from `eat_food`; Ouroboros on a self-bite death; Quick Snack on fast back-to-back eats).

**Not localized** (unlike Pong/Frogger): all strings are hardcoded English; there is no `assets/locales/`. The only asset is `assets/fonts/font.ttf`.

**Tests (38)**: `src/gameplay_tests.rs` (28 — direction/buffering, step/grow/death, food placement, chaos rules, versus resolution) plus inline `mod tests` in `spawning.rs` (5 — grid geometry) and `achievements.rs` (5 — registration/section parity). Everything is headless; new rules go into `rules.rs` precisely so they can be tested here.

## Coordinate and scale conventions

- World origin is screen center; window is 800×600 (`WIN_W`/`WIN_H`).
- The playfield is a 26×16 logical grid (`GRID_COLS`×`GRID_ROWS`) of 28-px cells (`CELL_PX`), centered on x = 0 and shifted down by `PLAYFIELD_OFFSET_Y` to leave a HUD band. **Column 0 is the left edge, row 0 the bottom** — `Direction::Up` is +y in both grid and world space. `cell_to_world` (spawning.rs) is the one grid→world mapping.
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — that's why sprite scales are `size_px / RENDER_UNIT` (`SEGMENT_PX`, `FOOD_PX`, wall bars).
- **This game uses no physics at all** — no `RigidBody`, no `Collider`, no `PhysicsSystem`. Walls are purely visual sprites; the wall *rule* is the bounds check in `step_snake`. So the engine footgun "colliders are absolute pixels and ignore `Transform2D.scale`" cannot bite here — but it will the moment anyone adds a collider, since every sprite in this game is sized via `scale`. F1 toggles the collider debug overlay anyway (convention parity with the other games; it draws nothing here).

## The Deion Re-skin (Phase G): Hot Dog!

Planned identity under the Deion pivot (the game is still the neon original today):

- New title **Hot Dog!**: the snake becomes a **wiener dog** — quite literally a wiener AND a dog (a dachshund that is a hot dog). Working name **"Frank"**, pending Jesse's sign-off; he's a NEW character who needs Jesse's design.
- His body grows a segment per food eaten; the growing body is the plan's excuse to introduce **tilemap logic** to this game — body segments rendered/tracked via the engine's `Tilemap` component as the dog stretches. Frogger is the engine's first Tilemap consumer; this becomes the second.
- The **angry meatball** — a shared cross-game character (the rocks in Meatieroids, the patty-layer rank in Burger Invaders) — roams the arena as a hazard here.
- This supersedes the earlier DEION_STYLE §5 Cubert-ice-cube casting. Arena theming (kitchen floor?), what the food pellets are, and versus-mode identity (two wiener dogs? Frank vs a rival?) are open.
- Style SSOT: `deion_assets/DEION_STYLE.md` via the root symlink (the symlink assumes the standard side-by-side checkout — the same requirement the Cargo path dep already imposes). Settled metrics: 16px base cell, nearest filtering, 5× integer scale to `RENDER_UNIT = 80`, one art cell = one world unit.
- Runtime assets arrive ONLY via the deion_assets sync copy into `assets/sprites/` (F2, not yet built) — never symlink or hand-copy art in. AI art is quarantined (`ai_` prefix, `deion_assets/ai/` only) and NEVER ships; `deion_assets/scripts/check_no_ai_assets.sh` must pass on shipping asset trees. Sheet clip names are the stable API.

## Review workflow

- The adversarial-review skill lives in `.claude/skills/`. Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` BEFORE implementation.
- Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh`; the `ADV_REVIEWED=1` prefix is allowed only after a code-mode review adjudicated with the user, or when the user explicitly skipped review.
- `review/` holds gitignored transients (only `.gitkeep` is tracked).
- NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies of `../../insiculous_2d/scripts/*` — re-copy when the engine master changes.
