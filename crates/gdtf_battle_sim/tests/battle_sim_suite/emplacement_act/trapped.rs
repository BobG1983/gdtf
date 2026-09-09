//! A one-sided seat whose one entry cell is held: the occupant cannot exit and cannot walk off.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{MoveRequested, MovementOccurred},
    ganger::Direction,
    metric::{Cell, CellLevel},
    situation::CoverSpawn,
    terrain::emplacement::EmplacementState,
    test_support::{SituationBuilder, test_terrain_registry},
};

use super::harness::*;

/// The seed both legs drive, so they differ only in whether the blocker is spawned.
const SEED: u64 = 0x5543_1239;

/// Ticks a walk is given to settle. Three steps and a frame to drop the walk fits easily.
const WALK_TICK_CAP: u32 = 16;

/// The seat's one rotated entry cell: North authored, turned East by the placed facing.
fn entry() -> CellLevel {
    ground(7, 5)
}

/// Where the blocker starts, one step east of the entry cell.
fn blocker_start() -> CellLevel {
    ground(8, 5)
}

/// Where both legs send the occupant: off the seat it is reachable only through [`entry`].
fn destination() -> CellLevel {
    ground(6, 2)
}

/// The one-sided seat on [`seat`], the actor on [`entry`], and a blocker only when asked for.
fn a_one_sided_seat(with_blocker: bool) -> (App, Entity, Entity) {
    let (mut app, seed) = battle_app(SEED);
    let mut terrain = test_terrain_registry();
    terrain.insert(ONE_SIDED, one_sided_emplacement());
    app.insert_resource(terrain);
    let mut spawns = vec![player_at(entry(), Direction::West)];
    if with_blocker {
        spawns.push(player_at(blocker_start(), Direction::West));
    }
    let situation = SituationBuilder::new()
        .with_gangers(spawns)
        .with_scatter(CoverSpawn::new(seat(), ONE_SIDED, PLACED_FACING))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat());
    let actor = player_on(&mut app, entry());
    (app, emplacement, actor)
}

/// Send `actor` to `dest`, tick until its walk settles, and report every step it took.
fn steps_taken(app: &mut App, actor: Entity, dest: CellLevel) -> Vec<MovementOccurred> {
    let _cleared: usize = drain_movements(app).len();
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    let mut taken: Vec<MovementOccurred> = Vec::new();
    for _ in 0..WALK_TICK_CAP {
        step(app, 1);
        taken.extend(
            drain_movements(app)
                .into_iter()
                .filter(|walked| walked.actor == actor),
        );
        if !is_walking(app, actor) {
            break;
        }
    }
    taken
}

/// The cells a run of steps went from and to, for a failure message that names them.
fn walked_cells(steps: &[MovementOccurred]) -> Vec<(Cell, Cell)> {
    steps
        .iter()
        .map(|walked| (walked.from, walked.to))
        .collect()
}

#[test]
fn a_held_entry_cell_leaves_the_occupant_of_a_one_sided_seat_stuck_in_it() {
    // Free entry cell: the same walk leaves by that cell and reaches the destination.
    let (mut app, emplacement, a) = a_one_sided_seat(false);
    mount(&mut app, a, emplacement);

    let steps = steps_taken(&mut app, a, destination());
    assert_eq!(
        steps.first().map(|walked| (walked.from, walked.to)),
        Some((seat().cell(), entry().cell())),
        "with its one entry cell free, the occupant's walk must leave the seat at {:?} by that \
         cell {:?}; it walked {:?}",
        seat(),
        entry(),
        walked_cells(&steps),
    );
    assert_eq!(
        pos_of(&app, a),
        Some(destination()),
        "the free-cell walk must reach {:?}, or the held leg below reads an unreachable \
         destination as a closed way out; it ended on {:?}",
        destination(),
        pos_of(&app, a),
    );

    // Held entry cell: the exit act is refused for it and the walk has no first step.
    let (mut app, emplacement, a) = a_one_sided_seat(true);
    let b = player_on(&mut app, blocker_start());
    mount(&mut app, a, emplacement);

    let blocking_steps = steps_taken(&mut app, b, entry());
    assert_eq!(
        pos_of(&app, b),
        Some(entry()),
        "PRECONDITION: B must reach the seat's one entry cell {:?}, or the trap was never set \
         and every assertion below passes for nothing; B stands on {:?} after walking {:?}",
        entry(),
        pos_of(&app, b),
        walked_cells(&blocking_steps),
    );

    let steps = steps_taken(&mut app, a, destination());
    assert!(
        steps.is_empty(),
        "the occupant must take NO step while B holds the seat's one entry cell {:?}; it walked \
         {:?}",
        entry(),
        walked_cells(&steps),
    );
    assert_eq!(
        pos_of(&app, a),
        Some(seat()),
        "the trapped occupant must still stand on the seat at {:?}; it stands on {:?}",
        seat(),
        pos_of(&app, a),
    );
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "a walk that takes no step vacates nothing, so the seat must read Occupied; it reads \
         {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(a),
        "the trapped occupant must still be the seat's occupant; the seat names {:?}",
        occupant(&app, emplacement),
    );
}
