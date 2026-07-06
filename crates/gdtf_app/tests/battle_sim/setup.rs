//! Battle setup: resources land + gangers spawn, failure paths, lifecycle cleanup,
//! and the empty situation.

use bevy::state::state::State;
use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState, GameState};
use gdtf_battle_sim::{
    armor::Wears,
    cover::CoverLedger,
    occupancy::OccupancyGrid,
    rng::ShotRng,
    situation::Situation,
    surface::SurfaceGrid,
    test_support::{SituationBuilder, ganger_at, key},
    tuning::CombatTuning,
    vertical::{LinkKind, VerticalLink, VerticalLinkGraph},
};
use gdtf_test_utils::advance_until;

use super::harness::*;

/// A situation with a DANGLING vertical link (an endpoint at a `(cell, level)` no
/// authored tile occupies) — `setup_battle` returns `Err(DanglingCell)` and inserts
/// no resource (the `setup_aborts_on_invalid_vertical_link` precedent). Built entirely
/// over the central
/// [`SituationBuilder`](gdtf_battle_sim::test_support::SituationBuilder), whose
/// [`vertical_link`](gdtf_battle_sim::test_support::SituationBuilder::vertical_link)
/// setter authors the deliberately-bad link this validation-abort test reaches for.
fn dangling_link_situation() -> Situation {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1); // never authored — the link dangles off it
    SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present) // only `present` authored; `missing` dangles
        .vertical_link(VerticalLink::new(present, missing, LinkKind::stair()))
        .build()
}

/// AC3 — `setup_battle` runs on the real `Commands` path: its four resources land
/// in the world, and the authored ganger count equals the spawned `Wears`-carrying
/// ganger count (the armor relationship) — proving the real setup ran, not a stub.
#[test]
fn setup_battle_lands_resources_and_spawns_gangers() {
    let situation = two_ganger_situation();
    let authored_gangers = situation.gangers.len();
    let mut app = walk_app(Some(situation));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates",
    );

    // All four setup_battle resources are present.
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "setup_battle must insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "setup_battle must insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "setup_battle must insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "setup_battle must insert a VerticalLinkGraph",
    );

    // The spawned ganger count equals the authored count (each ganger carries the
    // `Wears` armor relationship since GTW-323): exactly the fixture's gangers were
    // spawned on the real path. The ganger + its `Wears` armor-piece entities spawn as
    // `bsn!` scenes (`queue_spawn_related_scenes::<Wears>`), deferred to the `SpawnScene`
    // schedule; the related-piece spawn lands a frame after the ganger scene, so settle
    // one update so the `WornBy` back-reference hook has populated each ganger's `Wears`.
    app.update();
    let world = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        authored_gangers,
        "the spawned ganger count (each wearing armor via Wears) must equal the authored count",
    );
}

/// AC5 — a FAILED setup does NOT gate Generation complete (no panic, no silent
/// advance): a dangling vertical link makes `setup_battle` return `Err`; the plugin
/// logs it (no panic) and inserts NO resource, so the gate never fires, the state
/// stays in Generation, and no setup resource was inserted.
#[test]
fn failed_setup_does_not_advance_generation() {
    let mut app = walk_app(Some(dangling_link_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates",
    );

    // The marker must never appear (setup_battle aborted before inserting any
    // resource, so the gate's witness is absent).
    let advanced = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::Generation),
        BUDGET,
    );
    assert!(
        !advanced,
        "a failed setup must NOT advance past Generation; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::Generation),
        "the machine must remain in Generation when setup failed",
    );

    // No setup resource was inserted (validation aborts before any insert).
    assert!(
        app.world().get_resource::<CoverLedger>().is_none(),
        "a failed setup must insert no CoverLedger (it aborts before any resource insert)",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "a failed setup must insert no OccupancyGrid",
    );
}

/// AC6 — the battle-lifetime resources SURVIVE past Generation and are cleaned
/// ONLY on leaving the battle (`GameState::BattleScape`), while `CombatTuning`
/// (E10.4's persistent Load resource) is never touched.
#[test]
fn battle_resources_survive_battle_and_clean_on_exit() {
    let mut app = walk_app(Some(two_ganger_situation()));
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates",
    );

    // (a) Advance PAST Generation (into AnimateIn / BattleRunning) and assert the
    //     battle-lifetime resources all still exist — proving they survive past the
    //     Generation sub-state for the E10.6 acts — AND CombatTuning persists.
    let past_generation = advance_until(
        &mut app,
        |app| {
            matches!(
                battlescape_state(app),
                Some(BattleScapeState::AnimateIn | BattleScapeState::BattleRunning)
            )
        },
        BUDGET,
    );
    assert!(
        past_generation,
        "the walk should advance past Generation into AnimateIn/BattleRunning within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "RNG streams must survive past Generation (battle-lifetime)",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "CoverLedger must survive past Generation",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "SurfaceGrid must survive past Generation",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "OccupancyGrid must survive past Generation",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "VerticalLinkGraph must survive past Generation",
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning (E10.4's persistent Load resource) must still be present",
    );

    // (b) Advance until the machine has LEFT GameState::BattleScape, then assert the
    //     battle-lifetime resources are gone (cleaned at the battle boundary) while
    //     CombatTuning STILL persists (untouched by this plugin).
    //
    //     The battlescape now PERSISTS in BattleRunning (GTW-236, the placeholder budget
    //     auto-exit is gone), so insert the explicit `BattleRunningComplete` end-signal
    //     marker (standing in for the not-yet-wired victory/flee). Once the machine reaches
    //     BattleRunning the marker trips `move_on` and the chain advances out of the scape.
    app.world_mut().insert_resource(BattleRunningComplete);
    let left_battlescape = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<GameState>>()
                .is_none_or(|state| *state.get() != GameState::BattleScape)
        },
        BUDGET,
    );
    assert!(
        left_battlescape,
        "the walk should leave GameState::BattleScape within {BUDGET} updates",
    );
    assert!(
        app.world().get_resource::<ShotRng>().is_none(),
        "RNG streams must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_none(),
        "CoverLedger must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_none(),
        "SurfaceGrid must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "OccupancyGrid must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_none(),
        "VerticalLinkGraph must be cleaned on leaving the battle",
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must NOT be removed by this plugin (it is the persistent Load resource)",
    );
}

/// AC8 — the empty-`Situation` build keeps the deep walk green: with an EMPTY
/// `LoadedSituation` (the `Situation::default()` the GTW-261 gate now requires the
/// walk to seed), `setup_battle(&Situation::default())` returns Ok with zero gangers,
/// the four sim resources are inserted (empty grids), the gate fires, and Generation
/// ADVANCES (it does not hang). The landed `state_walk` deep walk covers the
/// reach-Teardown half; this asserts the empty-situation setup half (the same
/// zero-ganger build the `request_battle_setup` belt-and-suspenders fallback yields).
#[test]
fn empty_situation_builds_and_advances() {
    // The empty default LoadedSituation — the MinimalPlugins default-start path now
    // seeds it (GTW-261), exercising the zero-ganger battle build.
    let mut app = walk_app(None);
    assert!(
        drive_to_generation(&mut app),
        "the walk should reach Generation within {BUDGET} updates with the empty default situation",
    );

    // The Default (empty) situation still builds: the four sim resources are inserted.
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "the empty Default situation must still insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "the empty Default situation must still insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "the empty Default situation must still insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "the empty Default situation must still insert a VerticalLinkGraph",
    );

    // No gangers spawned (empty battlefield).
    let world = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        0,
        "the empty Default situation spawns zero gangers",
    );

    // And Generation still ADVANCES (the gate fired on the empty setup's resources).
    let reached_animate_in = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateIn),
        BUDGET,
    );
    assert!(
        reached_animate_in,
        "the absent-Situation Default path must still advance Generation to AnimateIn; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}
