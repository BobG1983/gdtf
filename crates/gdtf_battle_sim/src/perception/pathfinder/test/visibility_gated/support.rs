//! Local corridor / occupant fixtures for the visibility-gated planning tests —
//! reached by each concern file via `use super::support::*;`.

use bevy::prelude::{Entity, World};

use super::super::support::{cell, grid_with};
use crate::{
    metric::CellLevel,
    occupancy::{OccupancyGrid, TerrainKind},
    visibility::FactionRelation,
};

/// A throwaway live [`Entity`] handle for an occupant fixture (a ganger standing in a
/// cell) — keeps the test free of `unwrap`/`expect`.
pub(super) fn spawn_entity() -> Entity {
    World::new().spawn_empty().id()
}

/// A resolver mapping the single occupant `entity` to `relation`, anything else to
/// [`FactionRelation::Other`] — the GTW-353 occupant-faction resolver, hand-seeded.
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

/// A 1-wide horizontal corridor on `y = 5`, `x = 0..=4` on storey 0: walls fence the
/// `y = 4` and `y = 6` rows so the ONLY route from `(0, 5)` to `(4, 5)` runs straight
/// along the corridor (no diagonal escape past the fence). Returns the grid.
pub(super) fn corridor() -> OccupancyGrid {
    let mut walls = Vec::new();
    for x in 0..=4 {
        walls.push((cell(x, 4, 0), TerrainKind::Wall));
        walls.push((cell(x, 6, 0), TerrainKind::Wall));
    }
    grid_with(&walls)
}

/// The five corridor cells `(0..=4, 5, 0)` — the only walkable cells of the corridor.
pub(super) fn corridor_cells() -> Vec<CellLevel> {
    (0..=4).map(|x| cell(x, 5, 0)).collect()
}
