//! Shared fixtures for the [`has_los`](crate::los::has_los) tests — the cell/key
//! builders, the per-field ganger-component fixtures the [`Observer`] / [`Target`]
//! borrow-views point at, the arbitrary cover-entry helper, and the corpse-predicate
//! helpers. Each concern file does `use super::support::*`. No `App`, no RNG — pure
//! hand-built grids (the sim-unit-test idiom).

pub(super) use bevy::{ecs::world::World, prelude::Entity};

pub(super) use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, LifeState, Position, Stance, StanceKind},
    los::{Observer, Target, can_see, has_los},
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, StairEyeOffset},
    surface::{SlabState, SurfaceGrid},
    tuning::{CombatTuning, ViewRange},
};

/// A `(cell, level)` key from raw coords.
pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// A [`Position`] component at `(x, y, level)` — a per-field ganger fixture the
/// borrow-views point at.
pub(super) fn position(x: i32, y: i32, level: u8) -> Position {
    Position::new(key(x, y, level))
}

/// A [`Stance`] component holding `kind`.
pub(super) fn stance(kind: StanceKind) -> Stance {
    Stance::new(kind)
}

/// A [`Facing`] component looking along `dir`.
pub(super) fn facing(dir: Direction) -> Facing {
    Facing::new(dir)
}

/// An arbitrary cover entry at `band` (NOT shipped magnitudes — the band is what the
/// clearance test reads; the HP/armor are arbitrary, the brittle-test rule).
pub(super) fn cover_entry(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(50),
        band,
        ArmorProtection::new(5),
        ArmorHardness::new(2),
    )
}

/// A spawned `Entity` from a throwaway `World` — a real Bevy handle, never a numeric
/// id (GTW-10 / GTW-12).
pub(super) fn spawn_entity() -> Entity {
    let mut world = World::new();
    world.spawn_empty().id()
}

/// Two **distinct** spawned `Entity` handles from the SAME `World` — distinct indices,
/// so `==` tells them apart. (`spawn_entity` called twice returns the same `0v0` from
/// two fresh worlds; a corpse test that marks ONE occupant dead needs genuinely
/// different handles.)
pub(super) fn spawn_two_entities() -> (Entity, Entity) {
    let mut world = World::new();
    (world.spawn_empty().id(), world.spawn_empty().id())
}

/// A no-op `is_dead` predicate — marks no occupant a corpse, so every living occupant
/// stops sight. The corpse-passthrough test passes its own closure instead.
pub(super) fn no_dead() -> impl Fn(Entity) -> bool {
    |_| false
}

/// Place a living ganger occupant at `at` with silhouette `band` published — the
/// march needs BOTH the occupant slot and the band to strike it (GTW-304).
pub(super) fn place_occupant(
    grid: &mut OccupancyGrid,
    at: CellLevel,
    entity: Entity,
    band: HeightBand,
) {
    grid.set_occupant(at, Some(entity));
    grid.set_occupant_band(at, Some(band));
}
