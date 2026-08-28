//! A mount blocks the sightline across its cell and never the sightline of its own occupant.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    ganger::Direction,
    los::Sighted,
    metric::CellLevel,
    situation::GangerSpawn,
    terrain::emplacement::EmplacementState,
    test_support::{SituationBuilder, emplacement_at},
};

use super::harness::*;

/// The cell the emplacement is seeded on, between the two gangers.
fn mount_cell() -> CellLevel {
    ground(6, 5)
}

/// The cell the occupant enters from, on the mount's world-west entry side.
fn west_cell() -> CellLevel {
    ground(5, 5)
}

/// The cell the other ganger stands on, three steps east of the mount.
fn east_cell() -> CellLevel {
    ground(9, 5)
}

/// One seeded emplacement with a player ganger either side of it.
fn sight_layout(seed: u64, pair: [GangerSpawn; 2]) -> App {
    let (mut app, seed) = battle_app(seed);
    let situation = SituationBuilder::new()
        .with_gangers(pair)
        .with_scatter(emplacement_at(mount_cell()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    app
}

/// The pair held as entities by spawn cell, before any enter moves one off its cell.
fn pair_by_spawn_cell(app: &mut App) -> (Entity, Entity) {
    (player_on(app, west_cell()), player_on(app, east_cell()))
}

/// Seat the west ganger on the mount, and fail unless it now stands on the mount's cell.
fn seat_the_west_ganger(app: &mut App, actor: Entity) {
    let emplacement = seated_emplacement(app, mount_cell());
    mount(app, actor, emplacement);
    assert_eq!(
        state(app, emplacement),
        Some(EmplacementState::Occupied),
        "PRECONDITION: the seat on {:?} is not manned, so nothing below is about a mounted \
         ganger at all",
        mount_cell(),
    );
    assert_eq!(
        pos_of(app, actor),
        Some(mount_cell()),
        "PRECONDITION: the occupant stands on {:?} rather than on the mount {:?}, so the probe \
         below runs from beside the mount and says nothing about a mounted ganger",
        pos_of(app, actor),
        mount_cell(),
    );
}

#[test]
fn a_mounted_ganger_sees_past_its_own_mount() {
    let mut app = sight_layout(
        0x5543_1485,
        [
            player_at(west_cell(), Direction::East),
            player_at(east_cell(), Direction::West),
        ],
    );
    let (actor, watcher) = pair_by_spawn_cell(&mut app);
    high_mount_band(&app, mount_cell());
    seat_the_west_ganger(&mut app, actor);

    assert_eq!(
        sighted(&app, actor, watcher),
        Sighted::new(true),
        "the mounted ganger on {:?} is blinded by the mount it mans, and cannot see the ganger \
         on {:?}",
        mount_cell(),
        east_cell(),
    );
}

#[test]
fn an_observer_elsewhere_sees_the_mounted_ganger() {
    let mut app = sight_layout(
        0x5543_1585,
        [
            player_at(west_cell(), Direction::East),
            player_at(east_cell(), Direction::West),
        ],
    );
    let (actor, watcher) = pair_by_spawn_cell(&mut app);
    high_mount_band(&app, mount_cell());
    seat_the_west_ganger(&mut app, actor);

    assert_eq!(
        sighted(&app, watcher, actor),
        Sighted::new(true),
        "the ganger on {:?} cannot see the ganger mounted on {:?}, aiming at the mount's own \
         band",
        east_cell(),
        mount_cell(),
    );
}

#[test]
fn an_unmanned_mount_blinds_the_crouching_pair_either_side_of_it() {
    let mut app = sight_layout(
        0x5543_1685,
        [
            crouching_player_at(west_cell(), Direction::East),
            crouching_player_at(east_cell(), Direction::West),
        ],
    );
    let (west, east) = pair_by_spawn_cell(&mut app);
    high_mount_band(&app, mount_cell());

    let emplacement = seated_emplacement(&mut app, mount_cell());
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "PRECONDITION: the seat on {:?} is manned, so a ganger, not the mount, could be what \
         stops the probes below",
        mount_cell(),
    );

    assert_eq!(
        sighted(&app, west, east),
        Sighted::new(false),
        "the crouching ganger on {:?} sees through the unmanned mount on {:?} to the ganger on \
         {:?}",
        west_cell(),
        mount_cell(),
        east_cell(),
    );
    assert_eq!(
        sighted(&app, east, west),
        Sighted::new(false),
        "the crouching ganger on {:?} sees through the unmanned mount on {:?} to the ganger on \
         {:?}",
        east_cell(),
        mount_cell(),
        west_cell(),
    );
}
