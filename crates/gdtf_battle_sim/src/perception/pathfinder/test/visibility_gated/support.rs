use bevy::prelude::{Entity, World};

use super::super::support::{cell, grid_with};
use crate::{
    metric::CellLevel,
    occupancy::{OccupancyGrid, TerrainKind},
    visibility::FactionRelation,
};

pub(super) fn spawn_entity() -> Entity {
    World::new().spawn_empty().id()
}

pub(super) fn resolve_as(
    entity: Entity,
    relation: FactionRelation,
) -> impl Fn(Entity) -> FactionRelation {
    move |e| {
        if e == entity {
            relation
        } else {
            FactionRelation::Other
        }
    }
}

pub(super) fn corridor() -> OccupancyGrid {
    let mut walls = Vec::new();
    for x in 0..=4 {
        walls.push((cell(x, 4, 0), TerrainKind::Wall));
        walls.push((cell(x, 6, 0), TerrainKind::Wall));
    }
    grid_with(&walls)
}

pub(super) fn corridor_cells() -> Vec<CellLevel> {
    (0..=4).map(|x| cell(x, 5, 0)).collect()
}
