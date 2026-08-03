//! Project entity BlocksVision components onto the occupancy grid.

use bevy::{
    ecs::{entity::Entity, system::SystemParam},
    platform::collections::HashMap,
    prelude::{Added, Changed, Deref, Local, Or, Query, RemovedComponents, ResMut},
};

use super::OccupancyGrid;
use crate::{
    cover::HeightBand,
    metric::CellLevel,
    terrain::entity::{BlocksVision, TerrainCell},
};

/// Filter for vision-blocking entities that were added or changed.
pub type VisionOccluderChanged = Or<(Added<BlocksVision>, Changed<BlocksVision>)>;

/// Map of cells that occlude vision, with their height band.
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct VisionBlocking(HashMap<CellLevel, HeightBand>);

impl VisionBlocking {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the occluder band for a cell.
    pub fn insert(&mut self, cell_level: CellLevel, band: HeightBand) {
        self.0.insert(cell_level, band);
    }

    /// Remove the occluder at a cell.
    pub fn remove(&mut self, cell_level: &CellLevel) {
        self.0.remove(cell_level);
    }
}

/// System param for changed/removed vision-blocking entities.
#[derive(SystemParam)]
pub struct VisionBlockingChanges<'w, 's> {
    changed:
        Query<'w, 's, (Entity, &'static TerrainCell, &'static BlocksVision), VisionOccluderChanged>,
    removed: RemovedComponents<'w, 's, BlocksVision>,
}

/// Keep the occupancy grid's vision-blocking map in sync with entity components.
pub fn project_vision_blocking(
    mut grid: ResMut<OccupancyGrid>,
    mut changes: VisionBlockingChanges,
    mut tracked: Local<HashMap<Entity, CellLevel>>,
) {
    for entity in changes.removed.read() {
        if let Some(cell) = tracked.remove(&entity) {
            grid.clear_vision_blocking(cell);
        }
    }
    for (entity, cell, blocks) in &changes.changed {
        grid.set_vision_blocking(**cell, **blocks);
        tracked.insert(entity, **cell);
    }
}
