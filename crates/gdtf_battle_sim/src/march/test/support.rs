//! Shared fixtures and re-exports for the [`march_vector`](crate::march::march_vector)
//! tests — the cell/center/height builders, the cover-entry helper, the
//! tuning-derived band fractions, the spawned-`Entity` helper, and the off-grid
//! shooter sentinel. Each concern file does `use super::support::*`.

pub(super) use bevy::{ecs::world::World, math::Vec3, prelude::Entity};

pub(super) use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    march::{MarchKind, march_vector},
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center},
    occupancy::{
        GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancyInput, OccupantPlacement, TerrainKind,
        TerrainPlacement,
    },
    surface::{SlabState, SurfaceGrid},
    tuning::{BandEdge, CombatTuning},
};

/// A `(cell, level)` key from raw coords.
pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// The continuous sim-unit center of `(x, y)` on storey `level`.
pub(super) fn center(x: i32, y: i32, level: u8) -> SimPos {
    cell_center(Cell::new(x, y), Level::new(level))
}

/// A `SimPos` at the center of `(x, y, level)` raised to `above_floor` within
/// that storey — the round's z is `level + above_floor`.
pub(super) fn at_height(x: i32, y: i32, level: u8, above_floor: f32) -> SimPos {
    SimPos::new(
        x as f32 + 0.5,
        y as f32 + 0.5,
        f32::from(level) + above_floor,
    )
}

/// An arbitrary cover entry at `band` (NOT shipped magnitudes — the band is what
/// the clearance test reads; the HP/armor are arbitrary).
pub(super) fn cover_entry(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(50),
        band,
        ArmorProtection::new(5),
        ArmorHardness::new(2),
    )
}

/// An above-floor fraction that classifies HIGH under the default band edges (a
/// HIGH round) — derived from the tuning, never a literal.
pub(super) fn high_above_floor(tuning: &CombatTuning) -> f32 {
    f32::midpoint(*tuning.projectile_band_edges.mid_high, 1.0)
}

/// An above-floor fraction that classifies LOW under the default band edges.
pub(super) fn low_above_floor(tuning: &CombatTuning) -> f32 {
    *tuning.projectile_band_edges.low_mid * 0.5
}

/// A spawned `Entity` from a throwaway `World` — a real Bevy handle, never a
/// numeric id (GTW-10 / GTW-12).
pub(super) fn spawn_entity() -> Entity {
    let mut world = World::new();
    world.spawn_empty().id()
}

/// A cell far off the grid so it never coincides with any real cell under test —
/// the "no shooter cell exception in play" sentinel.
pub(super) fn far_shooter() -> CellLevel {
    key(59, 59, 7)
}
