use bevy::prelude::{Deref, Entity};

use crate::{
    metric::CellLevel,
    occupancy::OccupancyGrid,
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Routable(bool);

impl Routable {
        #[must_use]
    pub const fn new(routable: bool) -> Self {
        Self(routable)
    }
}

pub struct PlanningView<'a, R>
where
    R: Fn(Entity) -> FactionRelation,
{
            squad:       &'a SquadVisibility,
            relation_of: R,
}

impl<'a, R> PlanningView<'a, R>
where
    R: Fn(Entity) -> FactionRelation,
{
            #[must_use]
    pub const fn new(squad: &'a SquadVisibility, relation_of: R) -> Self {
        Self { squad, relation_of }
    }

                                                                        #[must_use]
    pub fn is_routable(&self, cell: CellLevel, grid: &OccupancyGrid) -> Routable {
        if !*self.squad.is_cell_explored(&cell) {
            return Routable::new(false);
        }
        self.is_open(cell, grid)
    }

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
