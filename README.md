# Bratdog

Game 4 of the 20 Games Challenge, built on the sibling [`insiculous_2d`](../../insiculous_2d) engine: classic grid Snake, re-skinned into the Deion world as **Bratdog**. You are Frank, a bratwurst dachshund loose in the kitchen, eating snacks and growing longer with every bite, with a deforming grid over the floor, chaos modes, achievements, and a 2-player versus mode where two dogs share one kitchen. (The site still lists it as Insiculous Snake.)

## Running

```bash
cargo run                     # play
cargo run --features editor   # run inside the engine's scene editor
cargo test                    # 65 headless tests
```

The same build runs in the browser at [beinsiculous.com/playground/snake/](https://beinsiculous.com/playground/snake/): the game inside the editor, layout only — the rules are compiled in and nothing you change there persists.

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

In **1 Player** mode Frank listens to *both* control sets — WASD, arrows, and either gamepad all steer him. In **2 Player Versus**, P1 drives Frank and P2 the darker dog.

## Mechanics

- **Tick-based movement** on a 24×15 grid of 32 px cells: the dog advances one cell per tick (0.14s in Normal). Turns are buffered up to two ahead for the classic responsive feel; a 180° reversal is never possible, even across two buffered turns.
- **Eat to grow**: each snack — a pretzel, a cheese bite or a bacon bone, chosen at random — adds a segment and 10 points. Death by the counter's edge, biting your own tail — or the other dog.
- **2 Player Versus**: two dogs, one kitchen, shared snacks. Both dogs resolve each tick simultaneously from the same starting positions — mutual head-ons and pass-through swaps kill both. First death ends the round; lone survivor wins, simultaneous deaths draw.
- **Chaos modes** (picked before each run):
  - *Normal* — the classic kitchen; the counter bites back.
  - *Insane* — faster trot, and every snack makes it faster still.
  - *Ridiculous* — the walls open up and wrap around (their sides are drawn open to show they're portals) and two snacks are on the floor.
  - *Insiculous* — all of the above at once.
- **Achievements** (9, single-player only): length milestones (10/20/35), a length-15 "feast" per chaos mode, Ouroboros (bite your own tail), Quick Snack (eat twice within 1.5s). Saved to `saves/snake_achievements.json`.
- **Universal pause** (Esc/Start): the whole match freezes — grid, particles, timers — under the engine's standard pause overlay.

## The Deion Pivot: Bratdog

**Bratdog** is the game title (Jesse, Sep 9 2026): bratwurst meets bratty dog, growing absurdly long with every bite. The re-skin shipped with the art revamp's batch 9: Frank drawn top-down from a modular sheet — a head, a tail, straights and corners, laid cell by cell into an engine `Tilemap` that is rewritten every frame, the head wagging and a dazed pose held when he crashes; player 2 as Frank recoloured, told apart by lightness as well as hue; three snacks; and a kitchen floor ringed by the counter's edge, drawn open in the wrap modes. The art follows the settled metrics — one art pixel per window pixel, pixel-snapped, 32 px cells — and is AI-drawn stand-in art (quarantined, free builds only), synced from `deion_assets` (see `deion_assets/DEION_STYLE.md` via the repo symlink).

**Open questions** (answered questions move up into the theme spec above and get DELETED from this list — live-docs convention):

- The wiener dog's name — is "Frank" final?
- The angry meatball — the shared cross-game menace who plays the rocks in Meatieroids, roaming the kitchen as a hazard: patrol or chase? Planned as the batch after the re-skin.
