# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. (`AGENTS.md` is a symlink to this file.)

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # run all 65 tests (headless, finish in well under a second)
cargo test <test_name>        # run a single test
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` feature). The `deion_assets` symlink at the repo root (`-> ../deion_assets`) makes the same assumption.

## Architecture

This is a single-crate game (`insiculous_snake`), shipped as **Bratdog** — snake re-skinned into the Deion world: Frank the bratwurst dachshund in a kitchen, built on the in-house `insiculous_2d` ECS engine. `SnakeGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs` — `init()` loads the sheets and spawns the kitchen floor, its wall and the grid backdrop; achievements register in `register_achievements()`, which the engine calls before the window opens; `cargo run -- --achievements-manifest <path>` exports the list; `update()` runs once per frame. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`, and at `/playground/snake/` in the browser (the same feature, built by the engine's `build_wasm.sh --kind editor` and served from the site); no game code changes between the modes.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()` in main.rs: `TitleScreen` / `ModeSelect` / `Achievements` dispatch to handlers in `menu.rs`; everything else (`Playing`, `GameOver`) falls through to `update_gameplay()` in `gameplay/mod.rs`. Flow is Title (1 Player / 2 Player Versus / Achievements / Exit) → ModeSelect (which is the **chaos-mode** select — the player-count choice already happened on the title) → Playing ↔ GameOver. Match lifecycle (`start_game`, `reset_to_title`) lives in `gameplay/mod.rs`; every frame ends with `update_entity_visibility()` (gameplay sprites are hidden on menu screens) and `draw_ui()` (drawing.rs).

**Pure rules / entity wiring split.** All grid rules are plain-data functions in `src/gameplay/rules.rs` — `next_direction`, `step_snake`, `resolve_versus_step`, `versus_result`, `place_food`, `tick_interval`, `food_count`, `walls_wrap`, `versus_spawn`, `starting_body`, and the drawing rules `piece_for` (which of Frank's pieces a body cell shows, wrap seams included), `step_direction` and `snack_for`. No entities, no `GameContext`, fully headless-testable. The `SnakeGame` methods in `gameplay/mod.rs` turn their outcomes into entities, particles, grid ripples, and achievements. Keep new rules on the pure side of this line.

**Tick-based movement, not per-frame.** `tick_timer` counts down by `ctx.delta_time`; a `while tick_timer <= 0.0` loop calls `advance()` once per elapsed tick, so a slow frame can run multiple grid steps. The interval comes from `tick_interval(chaos_mode, total_foods_eaten())` — total across *both* snakes in versus, so either player's eating speeds the shared clock in Insane-family modes.

**The body is data; the map follows.** `SnakeState.cells` (a `VecDeque<IVec2>`, head at the front) is the source of truth. Movement = `push_front` new head + `pop_back` tail; eating skips the pop. Each dog owns one engine `Tilemap` (`SnakeState.map`), and **`body::retile_dog` is its only writer**: once per frame, after all of the frame's ticks, it rewrites the whole map from the cells, the play clock and the dog's death time — every cell's piece from `piece_for`, the head and tail wagging at the sheet's rate, a dead dog's head playing `hurt_<facing>` and holding its last frame while its tail stops. Two writers would race for the head tile (a tick redraw strobing it to frame 0). `spawning::map_row` is the one conversion between the game's rows (0 at the bottom) and a map's (0 at the top). All collision is grid-cell math — there is no physics anywhere in this game.

**Input buffering (the classic-feel subtlety).** Turns are queued per snake (`input_queue`, capped at `INPUT_QUEUE_CAP = 2`, no consecutive duplicates), one applied per tick. `next_direction` discards turns equal or opposite to the *current* heading and validates at apply time, so two buffered turns can never combine into a 180 reversal. In single player one snake listens to both players' controls (`PlayerId::P1` **and** `P2` — WASD, arrows, and either gamepad all steer); in versus each snake gets its own player slot. Gameplay reads only `ctx.players` `GameAction`s (Move*, Action1, Menu) — never raw key codes (F1 debug toggle excepted). Bindings persist to `saves/input_settings.json`.

**Versus resolution is simultaneous.** `resolve_versus_step` computes both snakes' steps from the *pre-step* board, so resolution is order-independent: both heads on the same cell = mutual `HeadOn`; adjacent heads swapping through each other both die (`OtherSnake` — each old head cell is still the other's body); a snake's vacating tail cell is safe to enter unless that snake is eating. Only survivors advance. First death ends the round (`versus_result`): lone survivor wins, simultaneous deaths draw. Dead snakes' bodies stay on screen behind the game-over panel. Achievements are single-player only — versus rounds never unlock.

**Food placement is deterministic.** `place_food(occupied, seed)` hashes the seed to a cell and linear-probes to the first free one; seeds come from `frame_count` (+ a salt per pellet). `place_food` returns `None` only on a full board (`spawn_missing_food` treats that as "player has won Snake" and stops). Food respawn during a versus eat happens *after* both snakes advance, so a fresh pellet avoids both final positions. The pellet's snack — pretzel, cheese bite or bacon bone, all worth the same — is `snack_for(seed)`, read from a salted hash's high bits: the same hash modulo 3 would stripe the snacks by column, since 24 is a multiple of 3.

**Chaos modes** (engine `ChaosMode`, meaning defined here — see `mode_hint` in menu.rs): **Insane** = faster base tick that shrinks by `INSANE_TICK_STEP` per food eaten down to `INSANE_TICK_MIN`; **Ridiculous** = wrap-around walls (edges teleport; the ring draws its open sides — portals, not hazards) + two pellets on the board; **Insiculous** = both (`is_insane()`/`is_ridiculous()` both fire). `ChaosTheme::for_mode` supplies the accent layer — the grid backdrop's colour, the menus, the banner, the particle counts; nothing drawn is tinted by it. `apply_theme` pushes it onto the backdrop and writes the ring for the mode at `init`, `start_game` and `reset_to_title`. The runtime selection is mirrored into `ctx.chaos_mode` (read-write, engine persists it).

**Pause** follows the engine's universal pattern (`PauseMenu` gate at the top of `update_gameplay`, pausable in `Playing` only): Resume skips the rest of the frame so the keypress can't leak; the play clock stops, so the wag freezes, and the engine holds the grid backdrop still under `time_scale`. The clock keeps running through game over, so a dead dog's hurt plays out behind the panel.

**Visuals:** everything is synced Deion art at one art pixel per window pixel, pixel-snapped (`.with_pixel_snap(true)`): the kitchen floor and its wall are `Tilemap`s spawned in `init` that stay up under the menus; each dog is a `Tilemap`; each pellet is a sprite bobbing through its `idle`, and eating one spawns a detached `collect` one-shot kept in `collects` (entity and cell) so a restart or a quit ends it and no fresh pellet lands on it while it plays. **One draw depth per sheet** (`constants.rs`), nested floor, wall, the two dogs, the three snacks — the snacks over the dogs, or the head that just ate one would hide its collect. The engine's `GridBackdrop` draws over every sprite (`GridDrawOrder::OverSprites`) at `BACKDROP_ALPHA` — the floor is an opaque map, so a lattice behind the sprites would never show; eat and death events ripple it. Particle presets live in `effects.rs`, coloured by the caller (the snack's, the dog's; the white 1×1 in `Sheets.white` is their texture). Player 2 is `frank_player_two`, Frank recoloured. Menus use `MenuPanel`/`MenuStyle` chrome.

**Editor naming:** every spawned entity gets a `Name` component ("Frank", "Frank (player 2)", "Snack", "Kitchen Floor", "Kitchen Wall") so the editor hierarchy reads well — keep this for new entities.

**All tuning lives in `src/constants.rs`** (grid dims, tick times, the sheets and their depths, colours, achievement thresholds) and all entity creation in `src/spawning.rs`. Values tuned live in the editor inspector must be copied back into constants.rs to persist.

**Paths:** assets and saves anchor to `engine_core::game_root!()` (main.rs), so `cargo run` works from any cwd. Achievements persist to `saves/snake_achievements.json`; the 9 definitions, grouped `DISPLAY_SECTIONS`, and unlock logic live in `achievements.rs` (length milestones + per-mode feasts unlock live from `eat_food`; Ouroboros on a self-bite death; Quick Snack on fast back-to-back eats). The ids keep the game's original names because the saves persist them; the titles and descriptions are Bratdog's. The high-score keys (`solo`, `versus`) are unchanged too, so a returning player's list mixes scores from the old 26×16 board with the 24×15 one — accepted, and a local list only.

**Not localized** (unlike Pong/Frogger): all strings are hardcoded English; there is no `assets/locales/`. The assets are `assets/fonts/font.ttf` and the synced sprites under `assets/sprites/`.

**Tests (65)**: `src/gameplay_tests.rs` (37 — direction/buffering, step/grow/death, food placement, chaos rules, versus resolution, the pieces and the wrap seams, the snacks) plus inline `mod tests` in `spawning.rs` (7 — the layout, the ring), `achievements.rs` (5) and `constants.rs` (1 — the startup cards), and `src/flow_tests.rs` (15 — the match through the engine's `GameHarness` over the synced art: every clip the game types is in its sidecar, the maps, the wag on tick frames, the hurt after a right-angle turn into a wall, a versus head-on, the ring per mode, the collect, quitting after a death, the HUD and banner clear of the ring, the depth ladder, fair versus spawns, pellets off playing collects, played-out collects forgotten, a held-back pellet returning). Everything is headless; new rules go into `rules.rs` precisely so they can be tested there.

## Coordinate and scale conventions

- World origin is screen center; window is 800×600 (`WIN_W`/`WIN_H`).
- The playfield is a 24×15 logical grid (`GRID_COLS`×`GRID_ROWS`) of 32-px cells (`CELL_PX`, Frank's pitch), ringed by a 16 px wall to an 800×512 arena centred in the window (`PLAYFIELD_OFFSET_Y` 0), leaving a 44 px band above for the HUD and below for the chaos banner. **Column 0 is the left edge, row 0 the bottom** — `Direction::Up` is +y in both grid and world space. `cell_to_world` (spawning.rs) is the one grid→world mapping.
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — a sheet's sprite scale is `SheetSpec::scale()` (its cell over `RENDER_UNIT`). A `Tilemap` places tiles in pixels from its anchor, the centre of its top-left tile.
- **This game uses no physics at all** — no `RigidBody`, no `Collider`, no `PhysicsSystem`. The wall is a purely visual map; the wall *rule* is the bounds check in `step_snake`. So the engine footgun "colliders are absolute pixels and ignore `Transform2D.scale`" cannot bite here — but it will the moment anyone adds a collider, since every sprite in this game is sized via `scale`. F1 toggles the collider debug overlay anyway (convention parity with the other games; it draws nothing here).

## The Deion Re-skin: Bratdog

Shipped by the art revamp's batch 9 (`coordination/art-revamp/plan.md` at the working-set root; Jesse's rulings R8–R11, the decisions D48–D57):

- **Frank** is a bratwurst dachshund drawn top-down from a modular sheet (`ai_frank`: four heads, four tails, two straights, four corners, four `hurt_` heads). **Versus is two dogs**: player 2 is `ai_frank_player_two`, Frank recoloured with an identical silhouette, darker by lightness so the two tell apart without relying on hue.
- **The pellets** are three snacks — pretzel, cheese bite, bacon bone — chosen at random, worth the same.
- **The arena** is a kitchen floor ringed by the counter's edge; the ring's sides are drawn open in the wrap modes.
- The in-game and window title is **BRATDOG**; the site keeps 'Insiculous Snake' and the slug `snake` until `insiculous_web#64` rules. The text title stays until `deion_assets#13` draws Bratdog's title art.
- Style SSOT: `deion_assets/DEION_STYLE.md` via the `deion_assets -> ../../deion_assets` symlink. Runtime art arrives ONLY via the sync (`python3 deion_assets/scripts/sync_sprites.py .`, pinned in `assets/sprites/sync.list`) — never symlink or hand-copy art in. AI art is quarantined (`ai_` prefix): it may ship in FREE web builds, never in paid builds; `deion_assets/scripts/check_no_ai_assets.sh` must pass on any paid release's asset tree. Sheet clip names are the stable API.

Still open: **Frank's final name** (a working name, Jesse's to settle), and **the angry meatball** — the roaming hazard, a shared cross-game character, planned as the batch after the re-skin.

## Work tracking

Open work lives on the **Studio Board** (https://github.com/orgs/beinsiculous/projects/1)
as issues in this repo. **Always pass `-R beinsiculous/snake`** — a bare `gh` command
resolves against the session's working directory, which is often the working-set root, so
it lists and files against the wrong repository.

```sh
gh issue list -R beinsiculous/snake
gh api repos/beinsiculous/snake/milestones --jq '.[] | "\(.title): \(.description)"'
```

Issues are grouped into **sprint milestones**; each description records the batch's
internal order and its gates. Take the next unblocked issue in a sprint, not an arbitrary
one. Claim by assigning yourself; close with `fixes beinsiculous/snake#N` in the commit.

**Unfinished work becomes an issue.** Anything you don't finish — work you deferred, debt
you created, a follow-up you spotted — is filed before you report done. Never buried in a
doc, never left as a bare `TODO:`, never dropped. The `file-issue` skill carries the shape;
`sprint-planning` groups issues into shippable batches.

## Review workflow

- The adversarial-review skill lives in `.claude/skills/`. Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` BEFORE implementation.
- Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh`; the `ADV_REVIEWED=1` prefix is allowed only after a code-mode review adjudicated with the user, or when the user explicitly skipped review.
- `review/` holds gitignored transients (only `.gitkeep` is tracked).
- NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies — the canonical ones live in the working-set root, not in `insiculous_2d`. Never edit a copy: fix the root's and re-copy, and `scripts/check-skill-parity.sh` there reports any repo that drifted.
