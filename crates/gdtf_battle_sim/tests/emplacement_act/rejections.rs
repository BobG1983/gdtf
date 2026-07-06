//! Enter-act gate rejections — unaffordable, non-adjacent, and already-occupied enters are
//! each a no-op: no charge, no mount, no force-eject.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    acts::EnterEmplacementRequested,
    ganger::Direction,
    metric::CellLevel,
    prelude::{Faction, Tu},
    terrain::emplacement::EmplacementState,
    test_support::SituationBuilder,
};

use super::harness::*;

// ── Gate rejections: unaffordable / non-adjacent / already-occupied ─────────────

/// An actor that cannot afford the enter leaf is rejected — the emplacement stays Vacant and its
/// (too-small) TU is untouched.
#[test]
fn unaffordable_enter_is_rejected_no_charge() {
    let (mut app, seed) = battle_app(0x5543_0B0B);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = spawn_emplacement(&mut app, ground(6, 5));
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    // Drain the actor's TU below the enter cost.
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

/// A non-adjacent actor is rejected — the emplacement stays Vacant and no TU is spent.
#[test]
fn non_adjacent_enter_is_rejected_no_charge() {
    let (mut app, seed) = battle_app(0x5543_0C0C);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    // The emplacement is THREE cells east — outside the 8-adjacent reach.
    let emplacement = spawn_emplacement(&mut app, ground(8, 5));
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
        "a non-adjacent enter does not man the emplacement (the 8-adjacency gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "a rejected (non-adjacent) enter spends NO TU",
    );
}

/// Re-entering an ALREADY-occupied emplacement with a second ganger is a no-op (no force-eject) —
/// the first occupant stays seated and the second spends no TU.
#[test]
fn enter_on_occupied_is_rejected_no_force_eject() {
    let (mut app, seed) = battle_app(0x5543_0D0D);
    // Two adjacent players: first at (5,5), second at (7,5), emplacement between them at (6,5).
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            player_at(ground(7, 5), Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emp_cell = ground(6, 5);
    let emplacement = spawn_emplacement(&mut app, emp_cell);

    // Resolve the two player entities by their cells.
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

    // The FIRST mans the emplacement.
    app.world_mut()
        .write_message(EnterEmplacementRequested::new(first, emplacement));
    step(&mut app, 3);
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "the first ganger mans the emplacement",
    );
    let second_tu_before = tu_of(&app, second);

    // The SECOND tries to enter the already-occupied emplacement.
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
