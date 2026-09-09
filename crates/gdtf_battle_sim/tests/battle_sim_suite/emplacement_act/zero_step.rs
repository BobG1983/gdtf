//! A committed walk interrupted before its first step keeps the seat and pays nothing.

use bevy::{
    app::App,
    prelude::{Entity, Messages},
};
use gdtf_battle_sim::{
    acts::{MoveRejected, MoveRequested, MovementOccurred, movement::ReactionShotFired},
    ganger::Direction,
    metric::CellLevel,
    terrain::emplacement::{EmplacementState, Mounted},
    test_support::{SituationBuilder, emplacement_at},
};

use super::harness::*;

/// The seed this case drives.
const SEED: u64 = 0x5543_1232;

/// Ticks the interrupted walk is given to settle.
const WALK_TICK_CAP: u32 = 16;

/// The exit leaf this case tunes: positive, and cheap enough to leave the route affordable.
const EXIT_TU: u8 = 5;

/// The cell the actor starts on, beside the seat.
fn start() -> CellLevel {
    ground(5, 5)
}

/// Where the interrupted walk is sent: three steps east, well clear of the seat.
fn destination() -> CellLevel {
    ground(9, 5)
}

/// Every `MoveRejected` and `MovementOccurred` one tick left behind.
struct Written {
    rejects: Vec<MoveRejected>,
    steps:   Vec<MovementOccurred>,
}

/// A battle with one player actor on [`start`] and one vacant all-sided emplacement on [`seat`].
fn a_seat_beside_the_actor() -> (App, Entity, Entity) {
    let (mut app, seed) = battle_app(SEED);
    set_exit_tu(&mut app, EXIT_TU);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(start(), Direction::East)])
        .with_scatter(emplacement_at(seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat());
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    (app, actor, emplacement)
}

/// Drain both buffers this tick, so nothing written early is lost before the run ends.
fn drain(app: &mut App) -> Written {
    let rejects: Vec<MoveRejected> = app
        .world_mut()
        .resource_mut::<Messages<MoveRejected>>()
        .drain()
        .collect();
    let steps: Vec<MovementOccurred> = app
        .world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect();
    Written { rejects, steps }
}

#[test]
fn a_walk_interrupted_before_its_first_step_leaves_the_seat_occupied_and_the_ganger_mounted() {
    let (mut app, actor, emplacement) = a_seat_beside_the_actor();
    mount(&mut app, actor, emplacement);
    let tu_before = tu_of(&app, actor);
    let _cleared: usize = drain(&mut app).steps.len();

    app.world_mut()
        .write_message(MoveRequested::new(actor, destination()));
    app.world_mut().write_message(ReactionShotFired::new(actor));
    let mut rejects: Vec<MoveRejected> = Vec::new();
    let mut steps: Vec<MovementOccurred> = Vec::new();
    for _ in 0..WALK_TICK_CAP {
        step(&mut app, 1);
        let written = drain(&mut app);
        rejects.extend(written.rejects);
        steps.extend(written.steps);
    }

    // Guards: the move was accepted, and the walk it committed never stepped.
    let mine: Vec<&MoveRejected> = rejects.iter().filter(|r| r.actor == actor).collect();
    assert!(
        mine.is_empty(),
        "the move must be accepted, or the seat is untouched for want of a walk rather than \
         because the walk never stepped: {mine:?}",
    );
    let stepped: Vec<&MovementOccurred> = steps.iter().filter(|s| s.actor == actor).collect();
    assert!(
        stepped.is_empty(),
        "the reaction shot must stop the walk before its first step, or a walk that stepped \
         and came back would read the same: {stepped:?}",
    );

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "a walk that never stepped must leave the seat Occupied, it reads {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(actor),
        "the seat must still name this actor as its occupant, it names {:?}",
        occupant(&app, emplacement),
    );
    assert!(
        app.world().get::<Mounted>(actor).is_some(),
        "the actor must still carry Mounted; it carries {:?}",
        app.world().get::<Mounted>(actor).is_some(),
    );
    assert!(
        mount_entity(&app, emplacement).is_some(),
        "the seat's mounted weapon must survive a walk that never stepped, it names {:?}",
        mount_entity(&app, emplacement),
    );
    assert!(
        wields_mount(&mut app, actor),
        "the actor must still wield the seat's MountedWeapon",
    );
    assert_eq!(
        pos_of(&app, actor),
        Some(seat()),
        "the actor must still stand on the seat at {:?}, it stands on {:?}",
        seat(),
        pos_of(&app, actor),
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "a walk that never stepped spends nothing; the pool went from {tu_before:?} to {:?}",
        tu_of(&app, actor),
    );
}
