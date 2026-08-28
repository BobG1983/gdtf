//! A destroyed emplacement puts its gunner back on the ground, by fire or by smash.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{FireRequested, MeleeRequested},
    entity::TerrainPieceKind,
    ganger::{Direction, Position},
    metric::CellLevel,
    occupancy_sync::TerrainPieceDestroyed,
    situation::CoverSpawn,
    terrain::{emplacement::EmplacementState, facing::TerrainFacing},
    test_support::{SituationBuilder, emplacement_at, single_mode, test_pieces},
};

use super::harness::*;

/// The seed every case drives, so a rerun differs only in what the case writes.
const SEED: u64 = 0x5543_1182;

/// The act writes the destroyed message on the first update; the ejection reads it on the second.
fn settle_destruction(app: &mut App) {
    app.update();
    app.update();
}

/// The whole result of an ejection: the seat gives everything up, the gunner stands on `landing`.
fn assert_ejected(app: &App, seat: &MannedSeat, landing: CellLevel) {
    assert_eq!(
        state(app, seat.emplacement),
        Some(EmplacementState::Vacant),
        "a destroyed emplacement gives its seat up; the state reads {:?}",
        state(app, seat.emplacement),
    );
    assert_eq!(
        occupant(app, seat.emplacement),
        None,
        "the occupant record goes with the seat; it names {:?}",
        occupant(app, seat.emplacement),
    );
    assert_eq!(
        entered_from(app, seat.emplacement),
        None,
        "the entered-from record goes with the seat; it still holds {:?}",
        entered_from(app, seat.emplacement),
    );
    assert_eq!(
        mount_entity(app, seat.emplacement),
        None,
        "the mounted weapon is despawned with the piece; the seat holds {:?}",
        mount_entity(app, seat.emplacement),
    );
    assert_eq!(
        firing_damage_type(app, seat.occupant),
        Some(OWN_DAMAGE_TYPE),
        "the ejected gunner fires its own carried gun again, not the mount; it would fire {:?}",
        firing_damage_type(app, seat.occupant),
    );
    assert_eq!(
        pos_of(app, seat.occupant),
        Some(landing),
        "the ejected gunner stands on {landing:?}; it stands on {:?}",
        pos_of(app, seat.occupant),
    );
}

/// The cell a plain cover piece stands on, with a ganger standing on top of it.
fn cover_cell() -> CellLevel {
    ground(2, 2)
}

/// The manned seat no message names.
fn quiet_seat() -> CellLevel {
    ground(6, 5)
}

/// The manned seat whose floor is destroyed, rather than its mount.
fn floored_seat() -> CellLevel {
    ground(10, 5)
}

/// The seat nobody mans.
fn empty_seat() -> CellLevel {
    ground(14, 5)
}

/// The four subjects one run reads: a covered ganger, two manned seats, and an empty seat.
struct Subjects {
    stander:        Entity,
    quiet:          Entity,
    quiet_gunner:   Entity,
    floored:        Entity,
    floored_gunner: Entity,
    empty:          Entity,
}

/// One world holding every kind of subject a destroyed-piece message can name.
fn four_subjects() -> (App, Subjects) {
    let (mut app, seed) = battle_app(SEED);
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(cover_cell(), Direction::East),
            player_at(ground(5, 5), Direction::East),
            player_at(ground(9, 5), Direction::East),
        ])
        .with_scatter(CoverSpawn::new(
            cover_cell(),
            test_pieces::COVER,
            TerrainFacing::default(),
        ))
        .with_scatter(emplacement_at(quiet_seat()))
        .with_scatter(emplacement_at(floored_seat()))
        .with_scatter(emplacement_at(empty_seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let stander = player_on(&mut app, cover_cell());
    let quiet = seated_emplacement(&mut app, quiet_seat());
    let floored = seated_emplacement(&mut app, floored_seat());
    let empty = seated_emplacement(&mut app, empty_seat());
    let quiet_gunner = player_on(&mut app, ground(5, 5));
    let floored_gunner = player_on(&mut app, ground(9, 5));
    mount(&mut app, quiet_gunner, quiet);
    mount(&mut app, floored_gunner, floored);
    (
        app,
        Subjects {
            stander,
            quiet,
            quiet_gunner,
            floored,
            floored_gunner,
            empty,
        },
    )
}

#[test]
fn only_an_occupied_emplacement_the_message_names_gives_up_its_gunner() {
    let (mut app, subjects) = four_subjects();
    let world = app.world_mut();
    world.write_message(TerrainPieceDestroyed::new(
        cover_cell(),
        TerrainPieceKind::Cover,
    ));
    world.write_message(TerrainPieceDestroyed::new(
        floored_seat(),
        TerrainPieceKind::Slab,
    ));
    world.write_message(TerrainPieceDestroyed::new(
        empty_seat(),
        TerrainPieceKind::Emplacement,
    ));
    app.update();

    assert_eq!(
        pos_of(&app, subjects.stander),
        Some(cover_cell()),
        "a destroyed cover piece reaches no emplacement, so the ganger standing on it is not \
         moved; it stands on {:?}",
        pos_of(&app, subjects.stander),
    );
    assert_eq!(
        state(&app, subjects.quiet),
        Some(EmplacementState::Occupied),
        "an emplacement no message names keeps its gunner; its state reads {:?}",
        state(&app, subjects.quiet),
    );
    assert_eq!(
        pos_of(&app, subjects.quiet_gunner),
        Some(quiet_seat()),
        "that gunner is still on its seat; it stands on {:?}",
        pos_of(&app, subjects.quiet_gunner),
    );
    assert_eq!(
        state(&app, subjects.floored),
        Some(EmplacementState::Occupied),
        "a Slab message is the floor under the seat, not the mount, so the seat keeps its \
         gunner; its state reads {:?}",
        state(&app, subjects.floored),
    );
    assert_eq!(
        pos_of(&app, subjects.floored_gunner),
        Some(floored_seat()),
        "that gunner is still on its seat; it stands on {:?}",
        pos_of(&app, subjects.floored_gunner),
    );
    assert_eq!(
        state(&app, subjects.empty),
        Some(EmplacementState::Vacant),
        "a vacant emplacement has no gunner to set down and stays vacant; its state reads {:?}",
        state(&app, subjects.empty),
    );
}

#[test]
fn a_mount_shot_apart_sets_its_gunner_down_on_the_cell_it_entered_from() {
    let (mut app, seed) = battle_app(SEED);
    let shooter_cell = ground(9, 5);
    let seat = manned_seat(
        &mut app,
        seed,
        vec![shooter_at(shooter_cell, Direction::West)],
    );
    let shooter = player_on(&mut app, shooter_cell);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.3, 1),
        seat_cell().cell(),
        seat_cell().level(),
    ));
    settle_destruction(&mut app);

    assert!(
        mount_destroyed(&app, seat_cell()),
        "PRECONDITION: the round must fell the mount at {:?}, or nothing below is about a \
         destroyed emplacement",
        seat_cell(),
    );
    assert_ejected(&app, &seat, entry_cell());
}

#[test]
fn a_mount_smashed_apart_sets_its_gunner_down_on_the_cell_it_entered_from() {
    let (mut app, seed) = battle_app(SEED);
    let attacker_cell = ground(6, 4);
    let seat = manned_seat(
        &mut app,
        seed,
        vec![player_at(attacker_cell, Direction::South)],
    );
    let attacker = player_on(&mut app, attacker_cell);

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker, seat_cell()));
    settle_destruction(&mut app);

    assert!(
        mount_destroyed(&app, seat_cell()),
        "PRECONDITION: the smash must fell the mount at {:?}, or nothing below is about a \
         destroyed emplacement",
        seat_cell(),
    );
    assert_ejected(&app, &seat, entry_cell());
}

#[test]
fn a_gunner_whose_entry_cell_is_taken_is_set_down_on_the_mounts_own_cell() {
    let (mut app, seed) = battle_app(SEED);
    let attacker_cell = ground(6, 4);
    let bystander_cell = ground(4, 5);
    let seat = manned_seat(
        &mut app,
        seed,
        vec![
            player_at(attacker_cell, Direction::South),
            player_at(bystander_cell, Direction::East),
        ],
    );
    let attacker = player_on(&mut app, attacker_cell);
    let bystander = player_on(&mut app, bystander_cell);
    let Some(mut position) = app.world_mut().get_mut::<Position>(bystander) else {
        unreachable!("every seeded ganger carries a Position");
    };
    *position = Position::new(entry_cell());

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker, seat_cell()));
    settle_destruction(&mut app);

    assert_eq!(
        grid_occupant(&app, entry_cell()),
        Some(bystander),
        "PRECONDITION: the bystander must hold {:?} on the grid, or the scan reads it as free \
         and the case is the one above; the grid names {:?}",
        entry_cell(),
        grid_occupant(&app, entry_cell()),
    );
    assert!(
        mount_destroyed(&app, seat_cell()),
        "PRECONDITION: the smash must fell the mount at {:?}, or nothing below is about a \
         destroyed emplacement",
        seat_cell(),
    );
    assert_ejected(&app, &seat, seat_cell());
}
