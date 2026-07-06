//! The fire dispatch's [`SystemParam`] bundles + query aliases — the grid-resource
//! reads, the wielded-weapon marker probes, and the turn-to-fire query half.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, Query, Res, ResMut},
};

use crate::{
    cover::CoverLedger,
    fire::{BattleGrids, MeleeQuery, MountedQuery},
    ganger::{Facing, Tu},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
};

/// The change-driven world-grid resources [`dispatch_fire`](super::dispatch::dispatch_fire) reads, bundled into one
/// [`SystemParam`] so the system's parameter list stays under clippy's argument-count gate
/// (the [`BattleGrids`] / [`FireOrder`](crate::fire::FireOrder) grouping precedent in `fire.rs`).
///
/// Grouping the cohesive grid `Res<…>` reads into one param keeps [`dispatch_fire`](super::dispatch::dispatch_fire) at
/// seven parameters; the body assembles the borrow-based [`BattleGrids`] from these `Res`
/// reads via `BattleGridsParam::grids`. A transparent system-param bundle of existing
/// named world-state resources — not itself a wrapped domain scalar.
///
/// GTW-392: [`BraceStairCells`] is included so the terrain-brace gate in `resolve_round`
/// can consult the lower-endpoint stair-cell set without an additional system param.
#[derive(SystemParam)]
pub struct BattleGridsParam<'w> {
    /// The coarse 3D occupancy grid — the march's collision / occupant-band surface.
    occupancy:   Res<'w, OccupancyGrid>,
    /// The persistent floor/roof-slab + ground surface grid the march flies through.
    surface:     Res<'w, SurfaceGrid>,
    /// The model cover ledger — peeked (read) for the faced cell + the target cell's
    /// cover band, and **spent** (write) when a round strikes cover (GTW-364), so it is
    /// a [`ResMut`] now (the cover-hit depletion path writes the ledger's HP in place).
    cover:       ResMut<'w, CoverLedger>,
    /// The model slab ledger — **spent** (write) when a round strikes a floor/roof slab
    /// (GTW-365), so it is a [`ResMut`] too (the slab-hit depletion path writes the
    /// ledger's HP in place). The march reads slab *existence* from `surface` above.
    slab:        ResMut<'w, SlabLedger>,
    /// GTW-392: the lower-endpoint brace-stair-cell set — the terrain-brace gate reads
    /// this to decide whether a kneeling stair occupant earns the brace bonus.
    brace_cells: Res<'w, BraceStairCells>,
}

impl BattleGridsParam<'_> {
    /// Assemble the borrow-based [`BattleGrids`] [`fire`](crate::fire::fire) reads (and, GTW-364 / GTW-365,
    /// the cover- / slab-hit paths WRITE) from these grid resources — the occupancy /
    /// surface are read (`Res<T>` derefs to `&T`), the cover + slab ledgers are taken
    /// `&mut` (the depletion paths). Takes `&mut self` for the mutable ledger borrows;
    /// the grids are still never rebuilt — only the struck surface's HP is spent in place.
    pub(super) fn grids(&mut self) -> BattleGrids<'_> {
        BattleGrids {
            occupancy:   &self.occupancy,
            surface:     &self.surface,
            cover:       &mut self.cover,
            slab:        &mut self.slab,
            brace_cells: &self.brace_cells,
        }
    }

    /// The occupant entity (if any) at `at` — a single O(1) occupancy peek used for the
    /// GTW-328 fire declaration's resolved target. Borrows only the occupancy grid, so
    /// it does not conflict with the `&mut cover` borrow `grids` hands out.
    pub(super) fn occupant_at(&self, at: CellLevel) -> Option<Entity> {
        self.occupancy.occupant(&at)
    }
}

/// The two wielded-weapon MARKER probes [`dispatch_fire`](super::dispatch::dispatch_fire) resolves a shooter's PREFERRED ranged
/// weapon through, bundled into one [`SystemParam`] so the system stays under Bevy's 16-param
/// limit (the [`BattleGridsParam`] grouping precedent).
///
/// Both are cheap unit-item archetype-filter probes over the weapon entities: [`MeleeQuery`]
/// (GTW-505 — EXCLUDES the ganger's melee weapon from the ranged resolution) and [`MountedQuery`]
/// (GTW-543 — the emplacement's bolted-down gun the manning ganger PREFERS). Disjoint from the
/// stat-reading [`WeaponQuery`](crate::fire::WeaponQuery) and each other, so no `ParamSet` is needed. `dispatch_fire`
/// resolves the weapon as `mounted → ranged` (prefer the mount, else the carried gun) and threads
/// both borrows into [`fire`](crate::fire::fire) (which does the same internally).
#[derive(SystemParam)]
pub struct WeaponProbes<'w, 's> {
    /// The melee-weapon marker probe (GTW-505 C5) — the ranged resolution EXCLUDES a match.
    pub(super) melee:   MeleeQuery<'w, 's>,
    /// The mounted-weapon marker probe (GTW-543) — the ranged resolution PREFERS a match.
    pub(super) mounted: MountedQuery<'w, 's>,
}

/// The query the GTW-242 fire dispatch turns the shooter through for an out-of-arc shot —
/// the actor's `(&mut `[`Facing`]`, &mut `[`Tu`]`)`, the two components a turn-to-fire
/// mutates (set the new facing + debit the turn TU).
///
/// It conflicts with [`ShooterQuery`](crate::fire::ShooterQuery) (which reads `&Facing` and writes `&mut Tu`), so the
/// two CANNOT be independent system params — they are time-multiplexed through a
/// [`ParamSet`](bevy::ecs::system::ParamSet) (`bevy-traps.md` #3: a deliberate, ordered access of the same components at
/// distinct points in the system, never an ambiguous overlap). The turn write happens
/// FIRST (front-loaded), the [`ShooterQuery`](crate::fire::ShooterQuery) re-borrow for [`fire`](crate::fire::fire) SECOND.
pub(super) type TurnQuery<'world, 'state> =
    Query<'world, 'state, (&'static mut Facing, &'static mut Tu)>;
