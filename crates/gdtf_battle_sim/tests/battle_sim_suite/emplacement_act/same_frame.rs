//! An exit request and a move request in one frame: the exit leaf is charged once.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{ExitEmplacementRequested, MoveRequested},
    ganger::Direction,
    metric::CellLevel,
    terrain::emplacement::{EmplacementState, Mounted},
    test_support::{SituationBuilder, emplacement_at},
};

use super::harness::*;

/// The seed both runs drive, so they differ only in the messages written into the frame.
const SEED: u64 = 0x5543_1231;

/// The exit leaf both runs tune: positive, and cheap enough to leave the route affordable.
const EXIT_TU: u8 = 11;

/// Whether the frame that carries the move also carries an exit request for the same actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SameFrameExit {
    /// Both messages are written before the same `app.update()`.
    Written,
    /// Only the move is written.
    Absent,
}

/// What one run left behind: the TU it lost, and where the actor and its seat ended up.
struct Run {
    tu_lost:  u8,
    at:       Option<CellLevel>,
    state:    Option<EmplacementState>,
    occupant: Option<Entity>,
    mounted:  bool,
}

/// The cell the actor starts on, beside the seat.
fn start() -> CellLevel {
    ground(5, 5)
}

/// Where both runs send the walk: three steps east, well clear of the seat.
fn destination() -> CellLevel {
    ground(9, 5)
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

/// Step one tick at a time until the walk settles.
fn settle(app: &mut App, actor: Entity) {
    loop {
        step(app, 1);
        if !is_walking(app, actor) {
            break;
        }
    }
}

/// Mount the seat, send the actor to [`destination`], and report what the run left behind.
fn walk_off_the_seat(exit: SameFrameExit) -> Run {
    let (mut app, actor, emplacement) = a_seat_beside_the_actor();
    let leaf = exit_tu(&app);
    assert_eq!(
        leaf, EXIT_TU,
        "PRECONDITION: the run must hold the exit leaf it tuned, it reads {leaf}",
    );
    assert!(
        leaf > 0,
        "PRECONDITION: the exit leaf must be positive, or both runs spend the same TU whatever \
         order the exit and the walk run in; it reads {leaf}",
    );
    mount(&mut app, actor, emplacement);

    let before = tu_of(&app, actor).unwrap_or(0);
    if exit == SameFrameExit::Written {
        app.world_mut()
            .write_message(ExitEmplacementRequested::new(actor, emplacement));
    }
    app.world_mut()
        .write_message(MoveRequested::new(actor, destination()));
    settle(&mut app, actor);

    Run {
        tu_lost:  before.saturating_sub(tu_of(&app, actor).unwrap_or(0)),
        at:       pos_of(&app, actor),
        state:    state(&app, emplacement),
        occupant: occupant(&app, emplacement),
        mounted:  app.world().get::<Mounted>(actor).is_some(),
    }
}

#[test]
fn an_exit_and_a_move_in_one_frame_charge_the_exit_leaf_once() {
    let pair = walk_off_the_seat(SameFrameExit::Written);
    let move_alone = walk_off_the_seat(SameFrameExit::Absent);

    assert_eq!(
        pair.tu_lost, move_alone.tu_lost,
        "an exit request in the same frame as the move must charge the exit leaf once: the pair \
         lost {} TU against {} TU for the move alone, at an exit leaf of {EXIT_TU}",
        pair.tu_lost, move_alone.tu_lost,
    );
}

#[test]
fn an_exit_and_a_move_in_one_frame_still_walk_the_route() {
    let pair = walk_off_the_seat(SameFrameExit::Written);

    assert_eq!(
        pair.at,
        Some(destination()),
        "the walk must still reach {:?}; the actor was left on {:?}",
        destination(),
        pair.at,
    );
    assert_eq!(
        pair.state,
        Some(EmplacementState::Vacant),
        "the seat must be left Vacant, it reads {:?}",
        pair.state,
    );
    assert_eq!(
        pair.occupant, None,
        "the seat must name no occupant, it names {:?}",
        pair.occupant,
    );
    assert!(
        !pair.mounted,
        "the walker must carry no Mounted once it has left the seat; it carries {:?}",
        pair.mounted,
    );
}
