use bevy::ecs::{message::Messages, system::RunSystemOnce, world::World};

use super::super::systems::peek_population_needed;
use crate::{
    metric::{Cell, CellLevel, Level},
    occupancy_sync::TerrainPieceDestroyed,
    terrain::entity::TerrainPieceKind,
};

fn needed_after(kind: TerrainPieceKind) -> Option<bool> {
    let mut world = World::new();
    world.insert_resource(Messages::<TerrainPieceDestroyed>::default());
    world.write_message(TerrainPieceDestroyed::new(
        CellLevel::new(Cell::new(4, 4), Level::new(0)),
        kind,
    ));
    world.run_system_once(peek_population_needed).ok()
}

#[test]
fn a_destroyed_wall_needs_peeks_repopulated() {
    assert_eq!(
        needed_after(TerrainPieceKind::Wall),
        Some(true),
        "a destroyed wall changes which corners a stationary ganger can hug",
    );
}

#[test]
fn a_destroyed_cover_needs_peeks_repopulated() {
    assert_eq!(
        needed_after(TerrainPieceKind::Cover),
        Some(true),
        "a destroyed cover changes which corners a stationary ganger can hug",
    );
}

#[test]
fn a_destroyed_emplacement_needs_peeks_repopulated() {
    assert_eq!(
        needed_after(TerrainPieceKind::Emplacement),
        Some(true),
        "a destroyed emplacement changes which corners a stationary ganger can hug",
    );
}

#[test]
fn a_destroyed_slab_does_not_need_peeks_repopulated() {
    assert_eq!(
        needed_after(TerrainPieceKind::Slab),
        Some(false),
        "a slab is floor, not a corner to lean around, so its destruction must not trigger peek \
         repopulation",
    );
}
