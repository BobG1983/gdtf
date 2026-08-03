//! Planning view: which cells are routable given fog and occupants.

use bevy::prelude::{Deref, Entity};

use crate::{
    metric::CellLevel,
    occupancy::OccupancyGrid,
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

/// Whether a cell can be routed through.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Routable(bool);

impl Routable {
    #[must_use]
    pub const fn new(routable: bool) -> Self {
        Self(routable)
    }
}

/// View used by the pathfinder to decide which cells are open.
pub struct PlanningView<'a, R>
where
    R: Fn(Entity) -> FactionRelation,
{
    squad: &'a SquadVisibility,
    relation_of: R,
}

impl<'a, R> PlanningView<'a, R>
where
    R: Fn(Entity) -> FactionRelation,
{
    /// Build a planning view from squad fog and a faction-relation lookup.
    #[must_use]
    pub const fn new(squad: &'a SquadVisibility, relation_of: R) -> Self {
        Self { squad, relation_of }
    }

    /// Cell is explored and open (no path block, no visible enemy).
    #[must_use]
    pub fn is_routable(&self, cell: CellLevel, grid: &OccupancyGrid) -> Routable {
        if !*self.squad.is_cell_explored(&cell) {
            return Routable::new(false);
        }
        self.is_open(cell, grid)
    }

    /// Same as is_routable but skips the explored check (for vertical links).
    #[must_use]
    pub fn is_routable_link(&self, cell: CellLevel, grid: &OccupancyGrid) -> Routable {
        self.is_open(cell, grid)
    }

    fn is_open(&self, cell: CellLevel, grid: &OccupancyGrid) -> Routable {
        if *grid.is_path_blocked(&cell) {
            return Routable::new(false);
        }
        if let Some(occupant) = grid.occupant(&cell) {
            let relation = (self.relation_of)(occupant);
            if *is_ganger_visible(self.squad, &cell, relation) {
                return Routable::new(false);
            }
        }
        Routable::new(true)
    }
}
