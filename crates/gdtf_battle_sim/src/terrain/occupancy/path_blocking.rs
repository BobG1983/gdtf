//! Project entity `BlocksPathfinding` components onto the occupancy grid.

use bevy::{
    ecs::{entity::Entity, system::SystemParam},
    platform::collections::{HashMap, HashSet},
    prelude::{Added, Deref, Local, Query, RemovedComponents, ResMut},
};

use super::OccupancyGrid;
use crate::{
    metric::CellLevel,
    terrain::entity::{BlocksPathfinding, TerrainCell},
};

/// Set of cells currently marked path-blocked by entities.
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct PathBlocking(HashSet<CellLevel>);

impl PathBlocking {
    /// Empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a blocked cell.
    pub fn insert(&mut self, cell_level: CellLevel) {
        self.0.insert(cell_level);
    }

    /// Remove a blocked cell.
    pub fn remove(&mut self, cell_level: &CellLevel) {
        self.0.remove(cell_level);
    }
}

/// System param for added/removed path-blocking entities.
#[derive(SystemParam)]
pub struct PathBlockingChanges<'w, 's> {
    added:   Query<'w, 's, (Entity, &'static TerrainCell), Added<BlocksPathfinding>>,
    removed: RemovedComponents<'w, 's, BlocksPathfinding>,
}

/// Keep the occupancy grid's path-blocking set in sync with entity components.
pub fn project_path_blocking(
    mut grid: ResMut<OccupancyGrid>,
    mut changes: PathBlockingChanges,
    mut tracked: Local<HashMap<Entity, CellLevel>>,
) {
    for entity in changes.removed.read() {
        if let Some(cell) = tracked.remove(&entity) {
            grid.clear_path_blocking(cell);
        }
    }
    for (entity, cell) in &changes.added {
        grid.set_path_blocking(**cell);
        tracked.insert(entity, **cell);
    }
}
