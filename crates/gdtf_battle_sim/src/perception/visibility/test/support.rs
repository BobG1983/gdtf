//! grid builders that author/occupy candidate cells, and the corpse predicate. Each
pub(super) use bevy::{ecs::world::World, platform::collections::HashSet, prelude::Entity};

pub(super) use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, LifeState, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, StairEyeOffset},
    surface::SurfaceGrid,
    terrain::entity::TerrainPieceKind,
    tuning::{CombatTuning, ViewRange},
    visibility::{
        FactionRelation, FovObserver, SquadVisibility, accrue, is_ganger_visible, union_fov,
    },
};

pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

pub(super) fn position(x: i32, y: i32, level: u8) -> Position {
    Position::new(key(x, y, level))
}

pub(super) fn stance(kind: StanceKind) -> Stance {
    Stance::new(kind)
}

pub(super) fn facing(dir: Direction) -> Facing {
    Facing::new(dir)
}

pub(super) fn spawn_entity() -> Entity {
    let mut world = World::new();
    world.spawn_empty().id()
}

pub(super) fn no_dead() -> impl Fn(Entity) -> bool {
    |_| false
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

pub(super) fn place_occupant(
    grid: &mut OccupancyGrid,
    at: CellLevel,
    entity: Entity,
    band: HeightBand,
) {
    grid.set_occupant(at, Some(entity));
    grid.set_occupant_band(at, Some(band));
}

pub(super) fn alive_observer_at(x: i32, y: i32, level: u8) -> (Position, Stance, Facing) {
    (
        position(x, y, level),
        stance(StanceKind::Standing),
        facing(Direction::East),
    )
}
