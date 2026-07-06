//! Acts through the bootstrapped seam mutate the model: `FireRequested` wounds,
//! `SetStanceRequested` flips.

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

/// The stance the AC3 `SetStanceRequested` asks for — DISTINCT from [`AUTHORED_STANCE`],
/// so a successful flip is observable as a change to exactly this value.
const REQUESTED_STANCE: StanceKind = StanceKind::Prone;

/// AC2 — A `FireRequested` emitted inline in `BattleRunning` mutates the model. With the
/// shared builder rested in [`BattleScapeState::BattleRunning`], the drive proof QUERIES
/// the spawned shooter + target by [`Faction`] (off `app.world_mut()`, the test-body
/// idiom), ARMS the queried shooter via `entity_mut(..).insert(<weapon kit + TuMax +
/// Magazine>)` (a `GangerSpawn` authors no weapon), PUBLISHES the target's occupant band
/// in the live [`OccupancyGrid`] (the silhouette the band-free march reads to resolve a
/// `Ganger` hit), snapshots the target's `Hp`/`Wounds`/`LifeState`, emits a
/// [`FireRequested`] inline via the world message buffer, `update()`s once so the
/// GTW-212-gated battle-wide dispatch consumes it, and asserts ≥1 of
/// `Hp`/`Wounds`/`LifeState` changed — a landed hit, phrased as a before≠after relation,
/// never a pinned magnitude.
#[test]
fn fire_requested_in_battle_running_mutates_the_model() {
    let app_opt = bootstrap_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // The witness the GTW-212-gated battle-wide dispatch keys on is present in
    // BattleRunning, and the OccupancyGrid the march reads is too.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "the BattleInProgress witness must be present in BattleRunning (the dispatch gate)",
    );

    // Query the two SETUP-SPAWNED gangers (never hand-spawned): the shooter + the target.
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

    // ARM the queried shooter — insert the deterministic high-damage kit onto the
    // EXISTING setup-spawned entity (augment, never re-spawn), overwriting the registry
    // weapon so the test's single shot lands in a known regime.
    let mode = single_mode(0.2, 1);
    app.world_mut()
        .entity_mut(shooter)
        .insert(shooter_weapon_kit(mode));

    // PUBLISH the target's occupant band in the live grid — the band-free march reads it
    // to band the round vs the occupant. HIGH so a standing target is squarely in path.
    let (tx, ty, tl) = TARGET_AT;
    let target_at = key(tx, ty, tl);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    // Snapshot the target's battle surfaces before the emit.
    let hp_before = app.world().get::<Hp>(target).copied();
    let wounds_before = app.world().get::<Wounds>(target).copied();
    let life_before = app.world().get::<LifeState>(target).copied();

    // Emit the fire request IN BattleRunning, then advance one update so the gated
    // battle-wide dispatch (live across the battle) consumes it and runs fire().
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

/// AC3 — A `SetStanceRequested` flips its addressed component through the dispatch
/// boundary. Emitting a [`SetStanceRequested`] inline in `BattleRunning`, carrying the
/// actor [`Entity`] + a [`REQUESTED_STANCE`] DISTINCT from the [`AUTHORED_STANCE`] start,
/// then `update()`, mutates exactly that [`Stance`] on exactly that entity through the
/// GTW-212-gated, E10.2-owned dispatch + the landed `set_stance` verb. The test reads the
/// actor's [`Stance`] before and after, asserting it differs from the authored start
/// before and equals the requested value after — proving the message→dispatch→verb path
/// carries the actor [`Entity`] correctly (a relation, no tunable pinned).
#[test]
fn set_stance_requested_in_battle_running_flips_the_component() {
    let app_opt = bootstrap_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // Address the setup-spawned shooter as the posture actor (it carries Stance + Tu).
    let actor_found = find_ganger(&mut app, SHOOTER_FACTION);
    assert!(
        actor_found.is_some(),
        "the real setup must have spawned a faction-{SHOOTER_FACTION} shooter ganger",
    );
    let Some(actor) = actor_found else {
        return;
    };

    // The authored start stance is the distinct baseline the flip must move off of.
    let stance_before = app.world().get::<Stance>(actor).map(|s| **s);
    assert_eq!(
        stance_before,
        Some(AUTHORED_STANCE),
        "the spawned actor must hold the authored start stance before the request",
    );

    // Emit a stance request for a DISTINCT stance, then advance one update so the gated
    // dispatch consumes it and runs set_stance.
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
