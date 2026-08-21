use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, ExitEmplacementRequested, MoveRequested},
    ganger::{Direction, Position},
    metric::CellLevel,
    prelude::{Faction, Tu},
    terrain::emplacement::EmplacementState,
    test_support::{SituationBuilder, emplacement_at},
};

use super::harness::*;

#[test]
fn unaffordable_enter_is_rejected_no_charge() {
    let (mut app, seed) = battle_app(0x5543_0B0B);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .with_scatter(emplacement_at(ground(6, 5)))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, ground(6, 5));
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    let broke = enter_tu(&app).saturating_sub(1);
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(actor) {
        *tu = Tu::new(broke);
    }
    let tu_before = tu_of(&app, actor);

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "an unaffordable enter does not man the emplacement (the afford gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "a rejected (unaffordable) enter spends NO TU",
    );
    assert!(
        !wields_mount(&mut app, actor),
        "a rejected enter spawns no mount",
    );
}

#[test]
fn non_adjacent_enter_is_rejected_no_charge() {
    let (mut app, seed) = battle_app(0x5543_0C0C);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .with_scatter(emplacement_at(ground(8, 5)))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, ground(8, 5));
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    let tu_before = tu_of(&app, actor);

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "the actor at ground(5, 5) is not on an entry cell of the seat at ground(8, 5), so the \
         enter does not man it (the entry-sides gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "a rejected (non-adjacent) enter spends NO TU",
    );
}

#[test]
fn enter_on_occupied_is_rejected_no_force_eject() {
    let (mut app, seed) = battle_app(0x5543_0D0D);
    let emp_cell = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            player_at(ground(7, 5), Direction::West),
        ])
        .with_scatter(emplacement_at(emp_cell))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, emp_cell);

    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Faction, &gdtf_battle_sim::ganger::Position)>();
    let players: Vec<(Entity, CellLevel)> = q
        .iter(world)
        .filter(|(_, f, _)| ***f == PLAYER)
        .map(|(e, _, p)| (e, **p))
        .collect();
    let (Some(first), Some(second)) = (
        players
            .iter()
            .find(|(_, p)| *p == ground(5, 5))
            .map(|(e, _)| *e),
        players
            .iter()
            .find(|(_, p)| *p == ground(7, 5))
            .map(|(e, _)| *e),
    ) else {
        unreachable!("both players spawned at their authored cells");
    };

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(first, emplacement));
    step(&mut app, 3);
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "the first ganger mans the emplacement",
    );
    let second_tu_before = tu_of(&app, second);

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(second, emplacement));
    step(&mut app, 3);
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "re-entering an occupied emplacement does NOT displace the seated occupant (no force-eject)",
    );
    assert_eq!(
        tu_of(&app, second),
        second_tu_before,
        "a rejected (already-occupied) enter spends NO TU for the second ganger",
    );
}

/// The exit leaf the held-cell case runs, small enough that the mounted pool still covers it.
const HELD_CELL_EXIT_TU: u8 = 3;

/// Ticks the one-step walk onto the entry cell is given to settle.
const WALK_TICKS: u32 = 16;

/// The one player ganger standing on `at`, or a failure naming the cell and the count found.
fn player_on(app: &mut App, at: CellLevel) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction, &Position)>();
    let found: Vec<Entity> = query
        .iter(world)
        .filter(|(_, faction, position)| ***faction == PLAYER && ***position == at)
        .map(|(entity, ..)| entity)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one player ganger must stand on {at:?}, found {}",
        found.len(),
    );
    let [entity] = found[..] else {
        unreachable!("the count above is one");
    };
    entity
}

#[test]
fn an_exit_onto_a_cell_another_ganger_holds_is_refused_and_charges_nothing() {
    let (mut app, seed) = battle_app(0x5543_0E0E);
    set_exit_tu(&mut app, HELD_CELL_EXIT_TU);
    let (seat, entry, behind) = (ground(6, 5), ground(5, 5), ground(4, 5));
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(entry, Direction::East),
            player_at(behind, Direction::East),
        ])
        .with_scatter(emplacement_at(seat))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat);
    let (a, b) = (player_on(&mut app, entry), player_on(&mut app, behind));

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(a, emplacement));
    step(&mut app, 3);
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "PRECONDITION: A's enter must man the seat, or there is nothing to exit and every \
         assertion below passes for nothing; it reads {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(a),
        "PRECONDITION: the seat must name A as its occupant; it names {:?}",
        occupant(&app, emplacement),
    );

    app.world_mut().write_message(MoveRequested::new(b, entry));
    step(&mut app, WALK_TICKS);
    assert_eq!(
        pos_of(&app, b),
        Some(entry),
        "PRECONDITION: B must reach the cell A entered from at {entry:?}, or the collision this \
         case is about was never set up; B stands on {:?}",
        pos_of(&app, b),
    );

    let exit_cost = exit_tu(&app);
    assert!(
        exit_cost > 0,
        "the exit leaf must be a real positive cost, or an unchanged pool proves nothing",
    );
    let tu_before = tu_of(&app, a);
    assert!(
        tu_before.is_some_and(|pool| pool >= exit_cost),
        "A's pool must cover the exit leaf of {exit_cost}, or the refusal below is the \
         affordability term rather than the held cell; A holds {tu_before:?}",
    );

    app.world_mut()
        .write_message(ExitEmplacementRequested::new(a, emplacement));
    step(&mut app, 3);

    assert_ne!(
        pos_of(&app, a),
        pos_of(&app, b),
        "no exit may put two gangers on one cell: A stands on {:?} and B on {:?}",
        pos_of(&app, a),
        pos_of(&app, b),
    );
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "an exit refused for a held cell leaves the seat Occupied; it reads {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(a),
        "an exit refused for a held cell leaves A mounted; the seat names {:?}",
        occupant(&app, emplacement),
    );
    assert_eq!(
        pos_of(&app, a),
        Some(seat),
        "an exit refused for a held cell leaves A on the emplacement's cell {seat:?}; it stands \
         on {:?}",
        pos_of(&app, a),
    );
    assert_eq!(
        tu_of(&app, a),
        tu_before,
        "a refused exit spends NO TU: A's pool went from {tu_before:?} to {:?} against an exit \
         leaf of {exit_cost}",
        tu_of(&app, a),
    );
}
