//! All entity creation. Snake needs no physics — every entity is a plain
//! sprite (Name + Transform2D + Sprite) and all collision is grid-cell math.
//! Every entity gets a `Name` so the editor hierarchy reads well.

use engine_core::prelude::*;
use glam::IVec2;

use crate::constants::*;

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

/// Spawn one snake segment sprite at `cell`. The head glows brighter than
/// the body — the signature neon treatment for the player-controlled object.
pub(crate) fn spawn_segment(
    world: &mut World,
    tex: u32,
    cell: IVec2,
    color: Vec4,
    is_head: bool,
) -> EntityId {
    let emissive = if is_head { HEAD_EMISSIVE } else { BODY_EMISSIVE };
    let name = if is_head { "Head".to_string() } else { format!("Segment {cell}") };
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(
            cell_to_world(cell), 0.0, Vec2::splat(SEGMENT_PX / RENDER_UNIT)))
        .with(Sprite::new(tex).with_color(color).with_emissive(emissive))
        .id()
}

/// Spawn a food pellet sprite at `cell`.
pub(crate) fn spawn_food_sprite(world: &mut World, tex: u32, cell: IVec2) -> EntityId {
    world.spawn()
        .with(Name::new("Food"))
        .with(Transform2D::from_parts(
            cell_to_world(cell), 0.0, Vec2::splat(FOOD_PX / RENDER_UNIT)))
        .with(Sprite::new(tex).with_color(FOOD_COLOR).with_emissive(FOOD_EMISSIVE))
        .id()
}

/// Spawn the four wall bars framing the playfield. Purely visual — the wall
/// *rule* is the bounds check in `step_snake`.
pub(crate) fn spawn_walls(world: &mut World, tex: u32, color: Vec4) -> Vec<EntityId> {
    let field_w = GRID_COLS as f32 * CELL_PX;
    let field_h = GRID_ROWS as f32 * CELL_PX;
    // Bars overlap at the corners; extend the horizontals to cover them.
    let horizontal = Vec2::new(field_w + 2.0 * WALL_THICKNESS, WALL_THICKNESS);
    let vertical = Vec2::new(WALL_THICKNESS, field_h);
    let cx = 0.0;
    let cy = PLAYFIELD_OFFSET_Y;
    let bars = [
        ("Wall Top", Vec2::new(cx, cy + (field_h + WALL_THICKNESS) / 2.0), horizontal),
        ("Wall Bottom", Vec2::new(cx, cy - (field_h + WALL_THICKNESS) / 2.0), horizontal),
        ("Wall Left", Vec2::new(cx - (field_w + WALL_THICKNESS) / 2.0, cy), vertical),
        ("Wall Right", Vec2::new(cx + (field_w + WALL_THICKNESS) / 2.0, cy), vertical),
    ];
    bars.iter().map(|(name, pos, size)| {
        world.spawn()
            .with(Name::new(*name))
            .with(Transform2D::from_parts(*pos, 0.0, *size / RENDER_UNIT))
            .with(Sprite::new(tex).with_color(color).with_emissive(WALL_EMISSIVE))
            .id()
    }).collect()
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
    fn playfield_fits_inside_the_window_with_hud_room() {
        let top = cell_to_world(IVec2::new(0, GRID_ROWS - 1)).y + CELL_PX / 2.0;
        let bottom = cell_to_world(IVec2::new(0, 0)).y - CELL_PX / 2.0;
        let side = cell_to_world(IVec2::new(GRID_COLS - 1, 0)).x + CELL_PX / 2.0;
        assert!(top < WIN_H / 2.0 - 40.0, "HUD band above the field: top edge {top}");
        assert!(bottom > -WIN_H / 2.0 + 30.0, "banner room below the field: {bottom}");
        assert!(side < WIN_W / 2.0 - 10.0, "field must fit horizontally: {side}");
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

    #[test]
    fn spawned_segment_lands_on_its_cell() {
        let mut world = World::new();
        let cell = IVec2::new(5, 7);
        let seg = spawn_segment(&mut world, 0, cell, Vec4::ONE, true);
        let pos = world.get::<Transform2D>(seg).unwrap().position;
        assert!((pos - cell_to_world(cell)).length() < 0.001);
    }
}
