//! Walking off a seat: the dismount rides the commit, and the exit act it replaces does nothing.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{ExitEmplacementRequested, MoveRequested, MovementOccurred},
    ganger::Direction,
    metric::{Cell, CellLevel},
    situation::CoverSpawn,
    terrain::emplacement::{EmplacementState, Mounted},
    test_support::{SituationBuilder, emplacement_at, test_terrain_registry},
};

use super::harness::*;

/// The seed every case here drives, so two runs of one case differ only in what they tune.
const SEED: u64 = 0x5543_1226;

/// Ticks a walk is given to settle. Three steps and a frame to drop the walk fits easily.
const WALK_TICK_CAP: u32 = 16;

/// The cell the actor starts on beside the all-sided seat, and the one the exit act reverts to.
fn start() -> CellLevel {
    ground(5, 5)
}

/// Where the walk off the all-sided seat is sent: three steps east, well clear of the seat.
fn destination() -> CellLevel {
    ground(9, 5)
}

/// What one tick of a walk left behind: who is in the seat, where the actor stands, what it wrote.
struct Tick {
    occupant: Option<Entity>,
    at:       Option<CellLevel>,
    steps:    Vec<MovementOccurred>,
}

/// A battle with one player actor on [`start`] and one vacant all-sided emplacement on [`seat`].
///
/// `tune` runs before the battle is driven, which is where a case rewrites a tuning leaf.
fn a_seat_beside_the_actor(tune: impl FnOnce(&mut App)) -> (App, Entity, Entity) {
    let (mut app, seed) = battle_app(SEED);
    tune(&mut app);
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

/// The same battle with the one-sided emplacement, whose only rotated entry cell is `seat + (1,0)`.
fn a_one_sided_seat_beside_the_actor() -> (App, Entity, Entity) {
    let (mut app, seed) = battle_app(SEED);
    let mut terrain = test_terrain_registry();
    terrain.insert(ONE_SIDED, one_sided_emplacement());
    app.insert_resource(terrain);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(one_sided_entry(), Direction::West)])
        .with_scatter(CoverSpawn::new(seat(), ONE_SIDED, PLACED_FACING))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat());
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    (app, actor, emplacement)
}

/// The one-sided seat's single rotated entry cell: North authored, turned East by its facing.
fn one_sided_entry() -> CellLevel {
    let (cell, level) = seat().split();
    CellLevel::new(Cell::new(cell.x + 1, cell.y), level)
}

/// A cell the one-sided seat's shortest unconstrained route would leave by a different neighbour.
fn north_of_the_seat() -> CellLevel {
    ground(6, 2)
}

/// Send the actor to `dest` and step one tick at a time until the walk settles.
fn walk_to(app: &mut App, actor: Entity, emplacement: Entity, dest: CellLevel) -> Vec<Tick> {
    let _cleared: usize = drain_movements(app).len();
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    let mut ticks: Vec<Tick> = Vec::new();
    for _ in 0..WALK_TICK_CAP {
        step(app, 1);
        ticks.push(Tick {
            occupant: occupant(app, emplacement),
            at:       pos_of(app, actor),
            steps:    drain_movements(app),
        });
        if !is_walking(app, actor) {
            break;
        }
    }
    ticks
}

/// The first step this actor took after the move was committed, if it took one at all.
fn first_step(ticks: &[Tick], actor: Entity) -> Option<&MovementOccurred> {
    ticks
        .iter()
        .flat_map(|tick| tick.steps.iter())
        .find(|step| step.actor == actor)
}

/// The first cell this actor stood on that was not the seat, if it ever left it.
fn first_cell_off(ticks: &[Tick], from: CellLevel) -> Option<CellLevel> {
    ticks
        .iter()
        .filter_map(|tick| tick.at)
        .find(|at| *at != from)
}

#[test]
fn a_committed_move_vacates_the_seat_leaves_the_walker_where_it_walked_and_voids_the_exit_act() {
    let (mut app, actor, emplacement) = a_seat_beside_the_actor(|_untuned| {});
    mount(&mut app, actor, emplacement);

    let ticks = walk_to(&mut app, actor, emplacement, destination());

    // The commit vacated the seat, and nothing about the mount survived it.
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "the walk off the seat must leave the emplacement Vacant, it reads {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        None,
        "the walk off the seat must clear the occupant record, it still names {:?}",
        occupant(&app, emplacement),
    );
    assert!(
        app.world().get::<Mounted>(actor).is_none(),
        "removing MountedBy clears the walker's Mounted; it still carries {:?}",
        app.world().get::<Mounted>(actor).is_some(),
    );
    assert_eq!(
        mount_entity(&app, emplacement),
        None,
        "the walk off the seat must clear the mounted-weapon record, it still names {:?}",
        mount_entity(&app, emplacement),
    );
    assert!(
        !wields_mount(&mut app, actor),
        "the walker must wield no MountedWeapon once it has left the seat",
    );

    // No tick held a mounted ganger standing off its seat.
    for (tick, sample) in ticks.iter().enumerate() {
        if let Some(seated) = sample.occupant {
            assert_eq!(
                sample.at,
                Some(seat()),
                "tick {tick}: the seat still names {seated:?} as its occupant while that actor \
                 stands on {:?}, not on the seat at {:?}",
                sample.at,
                seat(),
            );
        }
    }
    assert!(
        first_cell_off(&ticks, seat()).is_some(),
        "no tick ever put the actor off the seat at {:?}, so the guarded check above asserted \
         nothing: {:?}",
        seat(),
        ticks.iter().map(|sample| sample.at).collect::<Vec<_>>(),
    );

    // The walker stayed where it walked, and its first step left the seat.
    assert_eq!(
        pos_of(&app, actor),
        Some(destination()),
        "the walker must end on the cell the move named, {:?}",
        destination(),
    );
    assert_eq!(
        first_step(&ticks, actor).map(|step| step.from),
        Some(seat().cell()),
        "the first step after the commit must leave the SEAT at {:?}, not the cell the actor \
         entered from at {:?}",
        seat(),
        start(),
    );

    // The exit act applied afterwards does nothing at all.
    let exit_cost = exit_tu(&app);
    assert!(
        exit_cost > 0,
        "the exit leaf must be a real positive cost, or an unchanged pool proves nothing",
    );
    let (at_before, tu_before) = (pos_of(&app, actor), tu_of(&app, actor));
    app.world_mut()
        .write_message(ExitEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);
    assert_eq!(
        pos_of(&app, actor),
        at_before,
        "the exit act after a walk off must move the actor nowhere; it went from {at_before:?} \
         to {:?}, undoing the walk",
        pos_of(&app, actor),
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "the exit act after a walk off must spend nothing; the pool went from {tu_before:?} to \
         {:?} against an exit leaf of {exit_cost}",
        tu_of(&app, actor),
    );
}

/// Mount, walk to `destination`, and report the TU the actor lost over the walk alone.
fn tu_lost_walking_off(exit_tu_leaf: u8) -> u8 {
    let (mut app, actor, emplacement) =
        a_seat_beside_the_actor(|app| set_exit_tu(app, exit_tu_leaf));
    mount(&mut app, actor, emplacement);
    assert_eq!(
        exit_tu(&app),
        exit_tu_leaf,
        "the run must hold the exit leaf it was built for",
    );
    let before = tu_of(&app, actor).unwrap_or(0);
    let _ticks: usize = walk_to(&mut app, actor, emplacement, destination()).len();
    assert_eq!(
        pos_of(&app, actor),
        Some(destination()),
        "a walk truncated short of {:?} would read as a smaller charge; it ended on {:?}",
        destination(),
        pos_of(&app, actor),
    );
    before.saturating_sub(tu_of(&app, actor).unwrap_or(0))
}

/// The two exit leaves the charging case runs, both cheap enough for the walk to complete.
const CHEAP_EXIT_TU: u8 = 3;

/// The dearer of the two exit leaves the charging case runs.
const DEAR_EXIT_TU: u8 = 11;

#[test]
fn walking_off_charges_the_exit_leaf_once_on_top_of_the_route() {
    let cheap = tu_lost_walking_off(CHEAP_EXIT_TU);
    let dear = tu_lost_walking_off(DEAR_EXIT_TU);

    assert_eq!(
        u32::from(dear) - u32::from(cheap),
        u32::from(DEAR_EXIT_TU) - u32::from(CHEAP_EXIT_TU),
        "the same route to {:?} walked off the same seat must differ by exactly the exit leaf: \
         {cheap} TU at a leaf of {CHEAP_EXIT_TU} against {dear} TU at a leaf of {DEAR_EXIT_TU}",
        destination(),
    );
}

#[test]
fn a_mounted_walkers_route_leaves_by_the_seats_one_rotated_entry_cell() {
    let (mut app, actor, emplacement) = a_one_sided_seat_beside_the_actor();
    mount(&mut app, actor, emplacement);

    let ticks = walk_to(&mut app, actor, emplacement, north_of_the_seat());

    assert_eq!(
        first_cell_off(&ticks, seat()),
        Some(one_sided_entry()),
        "the route off a one-sided seat must leave by its single rotated entry cell {:?}; it \
         left by {:?} instead",
        one_sided_entry(),
        first_cell_off(&ticks, seat()),
    );
    assert_eq!(
        pos_of(&app, actor),
        Some(north_of_the_seat()),
        "the constrained route must still reach {:?}, or a route the actor could not walk is \
         being read as a constrained one; it ended on {:?}",
        north_of_the_seat(),
        pos_of(&app, actor),
    );
}
