//! Drawing a dog: the one writer of its body `Tilemap`.
//!
//! A dog's map is rewritten whole once per frame, after the frame's ticks, from its
//! cells, the match's play time and its death time. One writer means a tick that moves
//! the dog and the wag that animates its head can never race for the same tile.

use std::collections::VecDeque;

use engine_core::prelude::*;
use glam::IVec2;

use crate::gameplay::{hurt_clip, piece_for, Piece};
use crate::spawning::{clip_cell, map_row};

/// Rewrite `map` from `cells`, head first. A living dog's head and tail show their
/// clips' frames at `play_time`, so they wag; a dead dog's head plays its hurt from
/// `death_time`, facing the way its head piece faces, and holds the last frame, while
/// its tail stops on its first. A cell whose clip yields no frame keeps the tile it had
/// — unreachable with a sheet that declares every clip, which the tests hold. The map
/// is rewritten in place: a frame allocates nothing here.
pub(crate) fn retile_dog(
    map: &mut Tilemap,
    sheet: &SpriteSheet,
    cells: &VecDeque<IVec2>,
    play_time: f32,
    death_time: Option<f32>,
) {
    let width = map.width;
    let slot_of = |cell: IVec2| (map_row(cell.y) * width + cell.x as u32) as usize;
    let shown = |index: usize| {
        let toward_head = index.checked_sub(1).map(|headward| cells[headward]);
        let piece = piece_for(toward_head, cells[index], cells.get(index + 1).copied())?;
        let (clip, elapsed) = match (piece, death_time) {
            (Piece::Head(facing), Some(died)) => (hurt_clip(facing), play_time - died),
            (Piece::Tail(_), Some(_)) => (piece.clip(), 0.0),
            _ => (piece.clip(), play_time),
        };
        clip_cell(sheet, clip, elapsed).map(|cell_index| cell_index + 1)
    };

    // The fallbacks are read before the clear; empty, as they always are with a whole
    // sheet, the list never allocates.
    let kept: Vec<(usize, u32)> = (0..cells.len())
        .filter(|&index| shown(index).is_none())
        .filter_map(|index| {
            let slot = slot_of(cells[index]);
            map.tiles.get(slot).map(|&tile| (slot, tile))
        })
        .collect();
    map.tiles.fill(0);
    for (index, &cell) in cells.iter().enumerate() {
        if let (Some(tile), Some(slot)) = (shown(index), map.tiles.get_mut(slot_of(cell))) {
            *slot = tile;
        }
    }
    for (slot, tile) in kept {
        map.tiles[slot] = tile;
    }
}
