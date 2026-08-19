use bevy::prelude::Entity;
use gdtf_battle_sim::{
    acts::EnterEmplacementRequested,
    ganger::Direction,
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
