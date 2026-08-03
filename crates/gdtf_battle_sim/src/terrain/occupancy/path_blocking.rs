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

#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct PathBlocking(HashSet<CellLevel>);

impl PathBlocking {
        #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

            pub fn insert(&mut self, cell_level: CellLevel) {
        self.0.insert(cell_level);
    }

                pub fn remove(&mut self, cell_level: &CellLevel) {
        self.0.remove(cell_level);
    }
}

#[derive(SystemParam)]
pub struct PathBlockingChanges<'w, 's> {
            added:   Query<'w, 's, (Entity, &'static TerrainCell), Added<BlocksPathfinding>>,
            removed: RemovedComponents<'w, 's, BlocksPathfinding>,
}

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
