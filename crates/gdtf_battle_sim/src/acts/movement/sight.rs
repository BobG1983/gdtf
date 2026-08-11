//! What a break-away sight check marches its ray through.

use bevy::prelude::Entity;

use crate::{
    cover::CoverLedger, march::MarchGrids, occupancy::OccupancyGrid, surface::SurfaceGrid,
    tuning::CombatTuning,
};

/// The grids, tuning and death test the break-away sight probe reads.
pub struct SightWorld<'a, F: Fn(Entity) -> bool> {
    occupancy: &'a OccupancyGrid,
    surface:   &'a SurfaceGrid,
    tuning:    &'a CombatTuning,
    is_dead:   F,
}

impl<'a, F: Fn(Entity) -> bool> SightWorld<'a, F> {
    /// Build from the grids the ray crosses, the tuning it is measured against, and a death test.
    #[must_use]
    pub const fn new(
        occupancy: &'a OccupancyGrid,
        surface: &'a SurfaceGrid,
        tuning: &'a CombatTuning,
        is_dead: F,
    ) -> Self {
        Self {
            occupancy,
            surface,
            tuning,
            is_dead,
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

    /// Whether an entity is a corpse, which a ray passes through.
    pub(crate) const fn is_dead(&self) -> &F {
        &self.is_dead
    }
}
