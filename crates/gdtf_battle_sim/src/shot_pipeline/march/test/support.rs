pub(super) use bevy::{ecs::world::World, math::Vec3, prelude::Entity};

pub(super) use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    march::{MarchDir, MarchKind, march_vector},
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center},
    occupancy::{
        BodyOcclusion, GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancyInput, OccupantPlacement,
        TerrainKind, TerrainPlacement,
    },
    surface::{SlabState, SurfaceGrid},
    terrain::entity::TerrainPieceKind,
    tuning::{BandEdge, CombatTuning},
};

pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

pub(super) fn center(x: i32, y: i32, level: u8) -> SimPos {
    cell_center(Cell::new(x, y), Level::new(level))
}

pub(super) fn at_height(x: i32, y: i32, level: u8, above_floor: f32) -> SimPos {
    SimPos::new(
        x as f32 + 0.5,
        y as f32 + 0.5,
        f32::from(level) + above_floor,
    )
}

pub(super) fn cover_entry(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(50),
        band,
        ArmorProtection::new(5),
        ArmorHardness::new(2),
        TerrainPieceKind::Cover,
    )
}

pub(super) fn high_above_floor(tuning: &CombatTuning) -> f32 {
    f32::midpoint(*tuning.projectile_band_edges.mid_high, 1.0)
}

pub(super) fn low_above_floor(tuning: &CombatTuning) -> f32 {
    *tuning.projectile_band_edges.low_mid * 0.5
}

pub(super) fn spawn_entity() -> Entity {
    let mut world = World::new();
    world.spawn_empty().id()
}

pub(super) fn spawn_two_entities() -> (Entity, Entity) {
    let mut world = World::new();
    (world.spawn_empty().id(), world.spawn_empty().id())
}

pub(super) fn far_shooter() -> CellLevel {
    key(59, 59, 7)
}

pub(super) fn no_dead() -> impl Fn(Entity) -> bool {
    |_| false
}
