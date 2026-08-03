pub(super) use bevy::{ecs::world::World, prelude::Entity};

pub(super) use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, LifeState, Position, Stance, StanceKind},
    los::{Observer, PeekOffset, Target, can_see, has_los, has_los_peeking},
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, StairEyeOffset},
    surface::{SlabState, SurfaceGrid},
    tuning::{CombatTuning, ViewRange},
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

pub(super) fn cover_entry(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(50),
        band,
        ArmorProtection::new(5),
        ArmorHardness::new(2),
    )
}

pub(super) fn spawn_entity() -> Entity {
    let mut world = World::new();
    world.spawn_empty().id()
}

pub(super) fn spawn_two_entities() -> (Entity, Entity) {
    let mut world = World::new();
    (world.spawn_empty().id(), world.spawn_empty().id())
}

pub(super) fn no_dead() -> impl Fn(Entity) -> bool {
    |_| false
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
