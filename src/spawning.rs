//! All entity creation. Snake needs no physics — every entity is a plain sprite or a
//! `Tilemap`, and all collision is grid-cell math. Every entity gets a `Name` so the
//! editor hierarchy reads well.
//!
//! Three kinds of map draw the board: the kitchen floor under everything, the wall's
//! ring around it, and one map per dog, rewritten from the dog's cells every frame.

use engine_core::prelude::*;
use glam::IVec2;

use crate::constants::*;
use crate::gameplay::Snack;
use crate::types::*;

/// World position of the center of grid cell (`col`, `row`).
/// Column 0 is the left edge, row 0 the bottom; the grid is centered on
/// x = 0 with the whole playfield shifted by `PLAYFIELD_OFFSET_Y`.
pub(crate) fn cell_to_world(cell: IVec2) -> Vec2 {
    Vec2::new(
        (cell.x as f32 - (GRID_COLS as f32 - 1.0) / 2.0) * CELL_PX,
        (cell.y as f32 - (GRID_ROWS as f32 - 1.0) / 2.0) * CELL_PX + PLAYFIELD_OFFSET_Y,
    )
}

/// True if `cell` lies inside the playfield.
pub(crate) fn in_bounds(cell: IVec2) -> bool {
    (0..GRID_COLS).contains(&cell.x) && (0..GRID_ROWS).contains(&cell.y)
}

/// The `Tilemap` row a grid row draws on. The game counts rows up from the bottom and a
/// map counts them down from the top; this is the one place the two meet.
pub(crate) fn map_row(row: i32) -> u32 {
    (GRID_ROWS - 1 - row) as u32
}

/// The sheet cell a clip shows `elapsed` seconds in, by the sidecar's rate. `None` for a
/// clip the sheet does not declare.
pub(crate) fn clip_cell(sheet: &SpriteSheet, clip: &str, elapsed: f32) -> Option<u32> {
    sheet.clips.iter().find(|(name, _)| name == clip).and_then(|(_, clip)| clip.cell_at(elapsed))
}

/// A grid-sized map drawing from `sheet`, empty, anchored on the top-left cell.
fn grid_map(sheet: &SpriteSheet, depth: f32) -> Tilemap {
    let mut map = Tilemap::new(GRID_COLS as u32, GRID_ROWS as u32, CELL_PX);
    map.tileset = sheet.texture.id;
    map.tile_uv_size = Vec2::new(1.0 / sheet.grid.cols as f32, 1.0 / sheet.grid.rows as f32);
    map.depth = depth;
    map
}

/// Spawn the kitchen floor: every grid cell its one tile.
pub(crate) fn spawn_floor(world: &mut World, sheet: &SpriteSheet) -> EntityId {
    let mut map = grid_map(sheet, FLOOR_DEPTH);
    let tile = clip_cell(sheet, FLOOR_CLIP, 0.0).map_or(1, |cell| cell + 1);
    map.tiles.fill(tile);
    world.spawn()
        .with(Name::new("Kitchen Floor"))
        .with(Transform2D::new(cell_to_world(IVec2::new(0, GRID_ROWS - 1))))
        .with(map)
        .id()
}

/// Spawn a dog's body map, empty until its first rewrite.
pub(crate) fn spawn_dog_map(world: &mut World, sheet: &SpriteSheet, dog: Dog) -> EntityId {
    world.spawn()
        .with(Name::new(dog.name()))
        .with(Transform2D::new(cell_to_world(IVec2::new(0, GRID_ROWS - 1))))
        .with(grid_map(sheet, dog_art(dog).depth))
        .id()
}

/// The ring's size in wall tiles: the grid in wall tiles, plus one tile each side.
pub(crate) const RING_COLS: u32 = (GRID_COLS * WALL_TILES_PER_CELL + 2) as u32;
pub(crate) const RING_ROWS: u32 = (GRID_ROWS * WALL_TILES_PER_CELL + 2) as u32;

/// The wall clip at ring tile (`col`, `row`), row 0 at the top: a corner at each corner,
/// a side along each edge — open in the wrap modes — and nothing inside the ring.
pub(crate) fn ring_clip(col: u32, row: u32, open: bool) -> Option<&'static str> {
    let (top, bottom) = (row == 0, row == RING_ROWS - 1);
    let (left, right) = (col == 0, col == RING_COLS - 1);
    let side = |closed: &'static str, opened: &'static str| Some(if open { opened } else { closed });
    match (top, bottom, left, right) {
        (true, _, true, _) => Some("corner_north_west"),
        (true, _, _, true) => Some("corner_north_east"),
        (_, true, true, _) => Some("corner_south_west"),
        (_, true, _, true) => Some("corner_south_east"),
        (true, ..) => side("north", "north_open"),
        (_, true, ..) => side("south", "south_open"),
        (_, _, true, _) => side("west", "west_open"),
        (.., true) => side("east", "east_open"),
        _ => None,
    }
}

/// Spawn the wall's ring, closed; `write_ring` sets it for the mode.
pub(crate) fn spawn_wall(world: &mut World, sheet: &SpriteSheet) -> EntityId {
    let mut map = Tilemap::new(RING_COLS, RING_ROWS, WALL_TILE_PX);
    map.tileset = sheet.texture.id;
    map.tile_uv_size = Vec2::new(1.0 / sheet.grid.cols as f32, 1.0 / sheet.grid.rows as f32);
    map.depth = WALL_DEPTH;
    write_ring(&mut map, sheet, false);
    let anchor = Vec2::new(
        -(RING_COLS as f32 - 1.0) * WALL_TILE_PX / 2.0,
        (RING_ROWS as f32 - 1.0) * WALL_TILE_PX / 2.0 + PLAYFIELD_OFFSET_Y,
    );
    world.spawn().with(Name::new("Kitchen Wall")).with(Transform2D::new(anchor)).with(map).id()
}

/// Write every ring tile for a closed or an open ring. The ring is never empty: an open
/// side is drawn, not removed, because a map has no alpha and a bare edge would show the
/// window's clear colour between the floor and the window's edge.
pub(crate) fn write_ring(map: &mut Tilemap, sheet: &SpriteSheet, open: bool) {
    for row in 0..RING_ROWS {
        for col in 0..RING_COLS {
            let tile = ring_clip(col, row, open)
                .and_then(|clip| clip_cell(sheet, clip, 0.0))
                .map_or(0, |cell| cell + 1);
            map.set_tile(col, row, tile);
        }
    }
}

/// Spawn the deforming grid drawn over the floor: the engine simulates and draws it,
/// gameplay events ripple it.
pub(crate) fn spawn_backdrop(world: &mut World, theme: &ChaosTheme) -> EntityId {
    world.spawn()
        .with(Name::new("Grid Backdrop"))
        .with(Transform2D::new(Vec2::ZERO))
        .with(GridBackdrop {
            draw_order: GridDrawOrder::OverSprites,
            color: crate::gameplay::backdrop_color(theme),
            ..GridBackdrop::default()
        })
        .id()
}

/// A detached effect's machine: it plays `clip` once and despawns.
fn one_shot_machine(clip: &str) -> ClipStateMachine {
    ClipStateMachine::new(clip, vec![(clip.to_string(), ClipState::new(clip, OnFinished::Despawn))])
}

/// A snack's one state: its idle bob.
fn snack_machine() -> ClipStateMachine {
    ClipStateMachine::new(SNACK_IDLE, vec![(SNACK_IDLE.to_string(), ClipState::staying(SNACK_IDLE))])
}

fn snack_components(sheet: &SpriteSheet, snack: Snack) -> (Transform2D, Sprite, SpriteAnimation) {
    let art = snack_art(snack);
    let sprite = sheet.sprite().with_offset(art.spec.sprite_offset()).with_depth(art.depth);
    (Transform2D::from_parts(Vec2::ZERO, 0.0, art.spec.scale()), sprite, sheet.animation())
}

/// Spawn a pellet: its snack at 1x, centred on its cell, bobbing.
pub(crate) fn spawn_snack(world: &mut World, sheet: &SpriteSheet, snack: Snack, cell: IVec2) -> EntityId {
    let (mut transform, sprite, animation) = snack_components(sheet, snack);
    transform.position = cell_to_world(cell);
    world.spawn()
        .with(Name::new("Snack"))
        .with(transform)
        .with(sprite)
        .with(animation)
        .with(snack_machine())
        .id()
}

/// Spawn the collect an eaten snack leaves on its cell: it plays once and ends itself;
/// the caller keeps it in `collects` so a restart or a quit can end it early, and so no
/// fresh pellet lands on its cell while it plays.
pub(crate) fn spawn_collect(world: &mut World, sheet: &SpriteSheet, snack: Snack, cell: IVec2) -> EntityId {
    let (mut transform, sprite, animation) = snack_components(sheet, snack);
    transform.position = cell_to_world(cell);
    world.spawn()
        .with(Name::new("Snack Collect"))
        .with(transform)
        .with(sprite)
        .with(animation)
        .with(one_shot_machine(SNACK_COLLECT))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_is_centered_horizontally() {
        let left = cell_to_world(IVec2::new(0, 0)).x;
        let right = cell_to_world(IVec2::new(GRID_COLS - 1, 0)).x;
        assert!((left + right).abs() < 0.001, "columns must mirror around x = 0");
    }

    #[test]
    fn adjacent_cells_are_one_cell_apart() {
        let a = cell_to_world(IVec2::new(3, 3));
        let b = cell_to_world(IVec2::new(4, 3));
        let c = cell_to_world(IVec2::new(3, 4));
        assert!((b.x - a.x - CELL_PX).abs() < 0.001);
        assert!((c.y - a.y - CELL_PX).abs() < 0.001, "row + 1 must move up in world space");
    }

    #[test]
    fn every_cell_and_ring_tile_centre_is_a_whole_pixel() {
        // Pixel snapping draws 1x art crisp only where an even-sized cell's centre lands on
        // a whole pixel.
        for col in 0..GRID_COLS {
            for row in 0..GRID_ROWS {
                let centre = cell_to_world(IVec2::new(col, row));
                assert_eq!(centre, centre.round(), "cell ({col}, {row})");
            }
        }
        let ring_anchor_x = -(RING_COLS as f32 - 1.0) * WALL_TILE_PX / 2.0;
        let ring_anchor_y = (RING_ROWS as f32 - 1.0) * WALL_TILE_PX / 2.0 + PLAYFIELD_OFFSET_Y;
        assert_eq!((ring_anchor_x, ring_anchor_y), (ring_anchor_x.round(), ring_anchor_y.round()));
    }

    #[test]
    fn the_ring_spans_the_window_and_meets_the_floor() {
        let floor_width = GRID_COLS as f32 * CELL_PX;
        let floor_height = GRID_ROWS as f32 * CELL_PX;
        assert_eq!(RING_COLS as f32 * WALL_TILE_PX, floor_width + 2.0 * WALL_TILE_PX);
        assert_eq!(RING_COLS as f32 * WALL_TILE_PX, WIN_W, "the arena spans the window's width");
        assert_eq!(RING_ROWS as f32 * WALL_TILE_PX, floor_height + 2.0 * WALL_TILE_PX);
        let arena_top = PLAYFIELD_OFFSET_Y + RING_ROWS as f32 * WALL_TILE_PX / 2.0;
        let arena_bottom = PLAYFIELD_OFFSET_Y - RING_ROWS as f32 * WALL_TILE_PX / 2.0;
        assert_eq!((WIN_H / 2.0 - arena_top, arena_bottom + WIN_H / 2.0), (44.0, 44.0), "the two bands");
    }

    #[test]
    fn map_row_flips_the_game_s_rows_onto_the_map_s() {
        assert_eq!(map_row(GRID_ROWS - 1), 0, "the top grid row is the map's first");
        assert_eq!(map_row(0), GRID_ROWS as u32 - 1, "the bottom grid row is the map's last");
    }

    #[test]
    fn the_ring_has_four_corners_four_sides_and_an_empty_middle() {
        for open in [false, true] {
            let suffix = if open { "_open" } else { "" };
            assert_eq!(ring_clip(0, 0, open), Some("corner_north_west"));
            assert_eq!(ring_clip(RING_COLS - 1, 0, open), Some("corner_north_east"));
            assert_eq!(ring_clip(0, RING_ROWS - 1, open), Some("corner_south_west"));
            assert_eq!(ring_clip(RING_COLS - 1, RING_ROWS - 1, open), Some("corner_south_east"));
            assert_eq!(ring_clip(7, 0, open).map(String::from), Some(format!("north{suffix}")));
            assert_eq!(ring_clip(7, RING_ROWS - 1, open).map(String::from), Some(format!("south{suffix}")));
            assert_eq!(ring_clip(0, 9, open).map(String::from), Some(format!("west{suffix}")));
            assert_eq!(ring_clip(RING_COLS - 1, 9, open).map(String::from), Some(format!("east{suffix}")));
            assert_eq!(ring_clip(1, 1, open), None, "inside the ring");
            assert_eq!(ring_clip(RING_COLS - 2, RING_ROWS - 2, open), None, "inside the ring");
        }
    }

    #[test]
    fn in_bounds_matches_grid_extents() {
        assert!(in_bounds(IVec2::new(0, 0)));
        assert!(in_bounds(IVec2::new(GRID_COLS - 1, GRID_ROWS - 1)));
        assert!(!in_bounds(IVec2::new(-1, 0)));
        assert!(!in_bounds(IVec2::new(0, GRID_ROWS)));
        assert!(!in_bounds(IVec2::new(GRID_COLS, 0)));
        assert!(!in_bounds(IVec2::new(0, -1)));
    }
}
