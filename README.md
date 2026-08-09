# Insiculous Snake

Game 4 of the 20 Games Challenge, built on the sibling [`insiculous_2d`](../../insiculous_2d) engine. Classic grid Snake in the project's neon Geometry-Wars look: glowing rects over a deforming spring-mass grid, particle bursts on every meal and death, chaos modes, achievements, and a 2-player versus mode on a shared board.

## Running

```bash
cargo run                     # play
cargo run --features editor   # run inside the engine's scene editor
cargo test                    # 38 headless tests
```

Requires the engine checkout side by side: `../../insiculous_2d` (relative path dependency).

## Controls

Bindings live in `saves/input_settings.json` (created on first run, hand-editable). Defaults:

| Action | Player 1 | Player 2 | Gamepad |
|---|---|---|---|
| Steer | WASD | Arrow keys | D-pad / left stick (P1 = pad 0, P2 = pad 1) |
| Confirm / restart | Space | Enter | (A) |
| Pause / back to title | Esc | Esc | Start |
| Menu navigation | W/S | Arrow Up/Down | D-pad (any pad) |
| Collider debug overlay | F1 | — | — |

In **1 Player** mode the lone snake listens to *both* control sets — WASD, arrows, and either gamepad all steer it. In **2 Player Versus**, P1 and P2 each drive their own snake.

## Mechanics

- **Tick-based movement** on a 26×16 grid: the snake advances one cell per tick (0.14s in Normal). Turns are buffered up to two ahead for the classic responsive feel; a 180° reversal is never possible, even across two buffered turns.
- **Eat to grow**: each pellet adds a segment, 10 points, and a particle pop. Death by wall, self-bite — or the other snake.
- **2 Player Versus**: two snakes, one board, shared food. Both snakes resolve each tick simultaneously from the same starting positions — mutual head-ons and pass-through swaps kill both. First death ends the round; lone survivor wins, simultaneous deaths draw.
- **Chaos modes** (picked before each run):
  - *Normal* — the classic garden; walls bite back.
  - *Insane* — faster slither, and every meal makes it faster still.
  - *Ridiculous* — walls wrap around (they dim to show they're portals) and two pellets are on the board.
  - *Insiculous* — all of the above at once.
- **Achievements** (9, single-player only): length milestones (10/20/35), a length-15 "feast" per chaos mode, Ouroboros (bite your own tail), Quick Snack (eat twice within 1.5s). Saved to `saves/snake_achievements.json`.
- **Universal pause** (Esc/Start): the whole match freezes — grid, particles, timers — under the engine's standard pause overlay.

## The Deion Pivot: Hot Dog!

The project is re-theming every game around **Deion the Insiculous** and his food-coded world (see `deion_assets/DEION_STYLE.md` via the repo symlink). Snake's planned identity:

**Hot Dog!** — the snake becomes a **wiener dog**: quite literally a wiener AND a dog, a dachshund that is a hot dog. Working name "Frank" (pending Jesse's sign-off — he's a new character who needs Jesse's design). His body grows a bun-warmed segment for every food eaten, and that ever-stretching body is the excuse to bring **tilemap logic** into this game: segments tracked and rendered through the engine's Tilemap component (Frogger was the engine's first Tilemap consumer; Hot Dog! becomes the second). Meanwhile the **angry meatball** — the shared cross-game menace who plays the rocks in Meatieroids and the patty-layer rank in Burger Invaders — roams the arena as a hazard.

This casting supersedes the earlier DEION_STYLE §5 idea of Cubert-as-ice-cube. Art follows the settled metrics: 16px base cells, nearest filtering, 5× integer scale (one 16px art cell = one 80px world unit). No AI-generated art ever ships.

**Open questions** (answered questions move up into the theme spec above and get DELETED from this list — live-docs convention):

- The wiener dog's name — is "Frank" final?
- His design — needs Jesse.
- Arena theme — kitchen floor? Something else?
- What are the food pellets?
- Versus identity — two wiener dogs, or Frank vs a rival?
- How do tilemap body segments map onto the existing grid logic (`cells` VecDeque → Tilemap tiles)?
- Angry meatball hazard behavior — patrol vs chase?
