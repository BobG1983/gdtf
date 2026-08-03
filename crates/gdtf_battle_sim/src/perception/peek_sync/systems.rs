use bevy::prelude::{Changed, MessageReader, Query, Res};

use super::corner::corner_lean;
use crate::{
    ganger::Position, los::PeekOffset, occupancy::OccupancyGrid, occupancy_sync::CoverDestroyed,
};

#[must_use]
pub fn peek_population_needed(
    moved: Query<(), Changed<Position>>,
    mut cover_destroyed: MessageReader<CoverDestroyed>,
) -> bool {
    let cover_changed = cover_destroyed.read().count() > 0;
    !moved.is_empty() || cover_changed
}

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
