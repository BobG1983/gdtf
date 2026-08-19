//! Keep peek offsets in sync when gangers move or cover is destroyed.

use bevy::prelude::{Changed, MessageReader, Query, Res};

use super::corner::corner_lean;
use crate::{
    ganger::Position, los::PeekOffset, occupancy::OccupancyGrid,
    occupancy_sync::TerrainPieceDestroyed, terrain::entity::TerrainPieceKind,
};

/// True when any ganger moved or a piece other than a slab was destroyed this frame.
#[must_use]
pub fn peek_population_needed(
    moved: Query<(), Changed<Position>>,
    mut destroyed: MessageReader<TerrainPieceDestroyed>,
) -> bool {
    // Read the whole buffer: short-circuiting would leave messages for the next frame.
    let mut cover_changed = false;
    for event in destroyed.read() {
        cover_changed |= event.kind != TerrainPieceKind::Slab;
    }
    !moved.is_empty() || cover_changed
}

/// Recompute each ganger's peek offset from their cell and the occupancy grid.
pub fn sync_peek_offsets(
    grid: Res<OccupancyGrid>,
    mut gangers: Query<(&Position, &mut PeekOffset)>,
) {
    for (position, mut current) in &mut gangers {
        let next = corner_lean(**position, &grid);
        if next != *current {
            *current = next;
        }
    }
}
