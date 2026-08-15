use gdtf_battle_sim::{
    acts::{FireRequested, SetStanceRequested},
    battle::BattleInProgress,
    cover::HeightBand,
    ganger::{Hp, LifeState, Stance, StanceKind, Wounds},
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    test_support::{key, single_mode},
};

use super::harness::*;

const REQUESTED_STANCE: StanceKind = StanceKind::Prone;

#[test]
fn fire_requested_in_battle_running_mutates_the_model() {
    let mut app = bootstrap_app();

    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "the BattleInProgress witness must be present in BattleRunning (the dispatch gate)",
    );

    let shooter_found = find_ganger(&mut app, SHOOTER_FACTION);
    let target_found = find_ganger(&mut app, TARGET_FACTION);
    assert!(
        shooter_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    assert!(
        target_found.is_some(),
        "the real setup must have spawned a faction-{TARGET_FACTION} target ganger",
    );
    let (Some(shooter), Some(target)) = (shooter_found, target_found) else {
        return;
    };
    assert_ne!(shooter, target, "shooter and target are distinct entities");

    let mode = single_mode(0.2, 1);
    app.world_mut()
        .entity_mut(shooter)
        .insert(shooter_weapon_kit(mode));

    let (tx, ty, tl) = TARGET_AT;
    let target_at = key(tx, ty, tl);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    let hp_before = app.world().get::<Hp>(target).copied();
    let wounds_before = app.world().get::<Wounds>(target).copied();
    let life_before = app.world().get::<LifeState>(target).copied();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(tx, ty),
        Level::new(tl),
    ));
    app.update();

    let hp_after = app.world().get::<Hp>(target).copied();
    let wounds_after = app.world().get::<Wounds>(target).copied();
    let life_after = app.world().get::<LifeState>(target).copied();

    let target_changed =
        hp_after != hp_before || wounds_after != wounds_before || life_after != life_before;
    assert!(
        target_changed,
        "a FireRequested in BattleRunning must land a hit — a target component changed (hp \
         {hp_before:?}->{hp_after:?}, wounds {wounds_before:?}->{wounds_after:?}, life \
         {life_before:?}->{life_after:?})",
    );
}

#[test]
fn set_stance_requested_in_battle_running_flips_the_component() {
    let mut app = bootstrap_app();

    let actor_found = find_ganger(&mut app, SHOOTER_FACTION);
    assert!(
        actor_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    let Some(actor) = actor_found else {
        return;
    };

    let stance_before = app.world().get::<Stance>(actor).map(|s| **s);
    assert_eq!(
        stance_before,
        Some(AUTHORED_STANCE),
        "the spawned actor must hold the authored start stance before the request",
    );

    app.world_mut()
        .write_message(SetStanceRequested::new(actor, REQUESTED_STANCE));
    app.update();

    let stance_after = app.world().get::<Stance>(actor).map(|s| **s);
    assert_eq!(
        stance_after,
        Some(REQUESTED_STANCE),
        "the SetStanceRequested must flip exactly the actor's Stance to the requested value \
         through the dispatch boundary (was {stance_before:?})",
    );
}
