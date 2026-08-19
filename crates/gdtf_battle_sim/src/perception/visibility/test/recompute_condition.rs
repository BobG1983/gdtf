use bevy::ecs::{message::Messages, system::RunSystemOnce, world::World};

use super::super::recompute::should_recompute_visibility;
use crate::{
    battle::BattleReady,
    metric::{Cell, CellLevel, Level},
    occupancy_sync::TerrainPieceDestroyed,
    terrain::entity::TerrainPieceKind,
};

// The system reads both buffers, and a missing one fails param validation.
fn bare_world() -> World {
    let mut world = World::new();
    world.insert_resource(Messages::<TerrainPieceDestroyed>::default());
    world.insert_resource(Messages::<BattleReady>::default());
    world
}

fn recompute_after(kind: TerrainPieceKind) -> Option<bool> {
    let mut world = bare_world();
    world.write_message(TerrainPieceDestroyed::new(
        CellLevel::new(Cell::new(6, 2), Level::new(1)),
        kind,
    ));
    world.run_system_once(should_recompute_visibility).ok()
}

#[test]
fn a_destroyed_wall_triggers_a_recompute() {
    assert_eq!(
        recompute_after(TerrainPieceKind::Wall),
        Some(true),
        "a destroyed wall opens a sightline, so squad FOV must be recomputed",
    );
}

#[test]
fn a_destroyed_cover_triggers_a_recompute() {
    assert_eq!(
        recompute_after(TerrainPieceKind::Cover),
        Some(true),
        "a destroyed cover opens a sightline, so squad FOV must be recomputed",
    );
}

#[test]
fn a_destroyed_slab_triggers_a_recompute() {
    assert_eq!(
        recompute_after(TerrainPieceKind::Slab),
        Some(true),
        "a destroyed slab opens a sightline between storeys, so squad FOV must be recomputed",
    );
}

#[test]
fn a_destroyed_emplacement_triggers_a_recompute() {
    assert_eq!(
        recompute_after(TerrainPieceKind::Emplacement),
        Some(true),
        "a destroyed emplacement opens a sightline, so squad FOV must be recomputed",
    );
}

#[test]
fn no_destruction_and_no_other_trigger_recomputes_nothing() {
    let mut world = bare_world();
    assert_eq!(
        world.run_system_once(should_recompute_visibility).ok(),
        Some(false),
        "with no destroyed piece, no moved observer, no occluder change and no BattleReady, \
         there is nothing to recompute from",
    );
}
