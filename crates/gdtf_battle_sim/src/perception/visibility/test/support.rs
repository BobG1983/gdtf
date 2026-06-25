//! Shared fixtures for the squad-visibility tests (GTW-340) — the `(cell, level)` key
//! builder, the observer-component fixtures the [`FovObserver`] view points at, the
//! grid builders that author/occupy candidate cells, and the corpse predicate. Each
//! concern file does `use super::support::*`. No `App`, no RNG — pure hand-built grids
//! (the sim-unit-test idiom).

pub(super) use bevy::{ecs::world::World, platform::collections::HashSet, prelude::Entity};

pub(super) use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, LifeState, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::{CombatTuning, ViewRange},
    visibility::{
        FactionRelation, FovObserver, SquadVisibility, accrue, is_ganger_visible, union_fov,
    },
};

/// A `(cell, level)` key from raw coords.
pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// A [`Position`] component at `(x, y, level)` — a per-field observer fixture the
/// [`FovObserver`] view points at.
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

/// A spawned `Entity` from a throwaway `World` — a real Bevy handle, never a numeric id.
pub(super) fn spawn_entity() -> Entity {
    let mut world = World::new();
    world.spawn_empty().id()
}

/// A no-op `is_dead` predicate — no occupant is a corpse, so every living occupant
/// stops sight.
pub(super) fn no_dead() -> impl Fn(Entity) -> bool {
    |_| false
}

/// An arbitrary cover entry at `band` (NOT shipped magnitudes — the band is what the
/// LOS/aim path reads; the HP/armor are arbitrary, the brittle-test rule).
pub(super) fn cover_entry(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(50),
        band,
        ArmorProtection::new(5),
        ArmorHardness::new(2),
    )
}

/// Place a living ganger occupant at `at` with silhouette `band` published — both the
/// occupant slot and the band, so the candidate cell is "occupied" AND its band reads
/// back (GTW-304).
pub(super) fn place_occupant(
    grid: &mut OccupancyGrid,
    at: CellLevel,
    entity: Entity,
    band: HeightBand,
) {
    grid.set_occupant(at, Some(entity));
    grid.set_occupant_band(at, Some(band));
}

/// Build a single conscious-Alive [`FovObserver`]-input bundle at `(x, y, level)`,
/// standing, facing East. Returns the owned components; the caller assembles the
/// borrow-view (the components must outlive the view).
pub(super) fn alive_observer_at(x: i32, y: i32, level: u8) -> (Position, Stance, Facing) {
    (
        position(x, y, level),
        stance(StanceKind::Standing),
        facing(Direction::East),
    )
}
