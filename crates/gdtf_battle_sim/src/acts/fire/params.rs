//! System params for fire dispatch (grids and weapon probes).

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, Query, Res, ResMut},
};

use crate::{
    cover::CoverLedger,
    fire::{BattleGrids, MeleeQuery, MountedQuery},
    ganger::{Facing, Tu},
    injuries::{InjuryRegistry, InjuryTables},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    rng::{InjuryRng, SeverityRng, ShotRng},
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

/// Occupancy, surface, cover, slab, and brace cells for fire.
#[derive(SystemParam)]
pub struct BattleGridsParam<'w> {
    occupancy:   Res<'w, OccupancyGrid>,
    surface:     Res<'w, SurfaceGrid>,
    cover:       ResMut<'w, CoverLedger>,
    slab:        ResMut<'w, SlabLedger>,
    brace_cells: Res<'w, BraceStairCells>,
}

impl BattleGridsParam<'_> {
    pub(super) fn grids(&mut self) -> BattleGrids<'_> {
        BattleGrids {
            occupancy:   &self.occupancy,
            surface:     &self.surface,
            cover:       &mut self.cover,
            slab:        &mut self.slab,
            brace_cells: &self.brace_cells,
        }
    }

    pub(super) fn occupant_at(&self, at: CellLevel) -> Option<Entity> {
        self.occupancy.occupant(&at)
    }
}

/// Combat tuning, injury content, and the RNG streams a shot rolls against.
#[derive(SystemParam)]
pub struct ShotRolls<'w> {
    pub(super) tuning:   Res<'w, CombatTuning>,
    pub(super) shot:     ResMut<'w, ShotRng>,
    pub(super) severity: ResMut<'w, SeverityRng>,
    pub(super) injury:   ResMut<'w, InjuryRng>,
    pub(super) tables:   Option<Res<'w, InjuryTables>>,
    pub(super) registry: Option<Res<'w, InjuryRegistry>>,
}

/// Melee and mounted weapon probes on the shooter.
#[derive(SystemParam)]
pub struct WeaponProbes<'w, 's> {
    pub(crate) melee:   MeleeQuery<'w, 's>,
    pub(crate) mounted: MountedQuery<'w, 's>,
}

pub(super) type TurnQuery<'world, 'state> =
    Query<'world, 'state, (&'static mut Facing, &'static mut Tu)>;
