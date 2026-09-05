//! What a break-away sight check marches its ray through.

use bevy::prelude::Entity;

use crate::{
    cover::CoverLedger, march::MarchGrids, occupancy::OccupancyGrid, surface::SurfaceGrid,
    tuning::CombatTuning,
};

/// The grids, tuning and floor test the break-away sight probe reads.
pub struct SightWorld<'a, F: Fn(Entity) -> bool> {
    occupancy:  &'a OccupancyGrid,
    surface:    &'a SurfaceGrid,
    tuning:     &'a CombatTuning,
    is_floored: F,
}

impl<'a, F: Fn(Entity) -> bool> SightWorld<'a, F> {
    /// Build from the grids the ray crosses, the tuning it is measured against, and a floor test.
    #[must_use]
    pub const fn new(
        occupancy: &'a OccupancyGrid,
        surface: &'a SurfaceGrid,
        tuning: &'a CombatTuning,
        is_floored: F,
    ) -> Self {
        Self {
            occupancy,
            surface,
            tuning,
            is_floored,
        }
    }

    /// Who stands where, and the extra eye height a stair cell gives.
    pub(crate) const fn occupancy(&self) -> &OccupancyGrid {
        self.occupancy
    }

    /// The grids one ray marches through, judged against this cover.
    pub(crate) const fn march(&self, cover: &'a CoverLedger) -> MarchGrids<'a> {
        MarchGrids {
            occupancy: self.occupancy,
            surface: self.surface,
            cover,
        }
    }

    /// Tuning the eye and aim heights come from.
    pub(crate) const fn tuning(&self) -> &CombatTuning {
        self.tuning
    }

    /// Whether a ray tests this entity at the floor band, whatever the grid last published.
    pub(crate) const fn is_floored(&self) -> &F {
        &self.is_floored
    }
}
