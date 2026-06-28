//! Tests for the battle-lifecycle DRIVER systems
//! [`setup_battle_on_request`](crate::battle::setup_battle_on_request) and
//! [`teardown_battle_on_request`](crate::battle::teardown_battle_on_request): the
//! seed-and-setup `Ok`/`Err` paths, the
//! [`BattleInProgress`](crate::battle::BattleInProgress) lifecycle, and the
//! teardown resource removal ([`CombatTuning`](crate::tuning::CombatTuning) untouched).

use bevy::prelude::{Commands, World};

use super::support::*;
use crate::rng::ReactionRng;

// === AC2 — SetupBattleRequested seeds the five RNG streams (from the message's seed)
// + runs setup_battle, emitting BattleReady on success; the seed threads through. ===

#[test]
fn setup_request_seeds_rng_inserts_resources_and_signals_ready() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();

    // The five battle-lifetime RNG streams were inserted (GTW-14).
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "the setup must insert ShotRng",
    );
    // The four setup_battle resources are present.
    assert!(
        app.world().get_resource::<CoverLedger>().is_some(),
        "the setup must insert a CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_some(),
        "the setup must insert a SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "the setup must insert an OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_some(),
        "the setup must insert a VerticalLinkGraph",
    );
    // GTW-226 AC2 — PlayerFaction is inserted on the SAME Ok path as
    // BattleInProgress (both present together) and, because the fixture omits
    // player_faction, defaults to gang 0.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "the setup must insert BattleInProgress on the Ok path",
    );
    let player = app.world().get_resource::<PlayerFaction>().copied();
    assert_eq!(
        player,
        Some(PlayerFaction::new(Faction::new(0))),
        "the setup must insert PlayerFaction seeded from the situation (default gang 0)",
    );
    // A BattleReady was emitted on success.
    assert_eq!(
        drain_battle_ready(&mut app),
        1,
        "a successful setup must emit exactly one BattleReady",
    );
}

#[test]
fn setup_threads_the_message_seed_through_rng_streams() {
    // Two runs with the SAME seed give a ShotRng whose first draw matches; a
    // DIFFERENT seed gives a different first draw — the message's seed threads
    // through ShotRng::from_root (a relation, never a pinned magnitude).
    let first_draw = |seed: u64| {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            two_ganger_situation(),
            BattleSeed::new(seed),
        ));
        app.update();
        app.world_mut()
            .get_resource_mut::<ShotRng>()
            .map(|mut rng| rng.next_u64())
    };
    assert_eq!(
        first_draw(SEED),
        first_draw(SEED),
        "the same seed must thread through to an identical ShotRng first draw",
    );
    assert_ne!(
        first_draw(SEED),
        first_draw(SEED ^ 0xFFFF),
        "a different seed must yield a different ShotRng first draw",
    );
}

// === AC3 — setup_battle runs on the REAL Commands path: the spawned ganger count
// (each carrying the `Wears` armor relationship) equals the authored ganger count,
// and the four resources are present. ===

#[test]
fn setup_runs_setup_battle_on_the_real_commands_path() {
    let situation = two_ganger_situation();
    let authored = situation.gangers.len();
    let mut app = headless_app();
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    // First update: the SetupBattleRequested drain runs setup_battle, which spawns each
    // ganger as a `bsn!` scene + queues its `Wears` armor-piece related scenes. The ganger
    // scene materializes on this update's SpawnScene; the related-piece spawn lands the
    // NEXT update, so settle a second update for the `WornBy` hook to populate `Wears`.
    app.update();
    app.update();

    let world = app.world_mut();
    // Since GTW-323 (ADR-0004) each spawned ganger carries the `Wears` armor
    // relationship (the armor stats live on related piece entities, not a `WornArmor`
    // component), so the spawned-ganger count is the count of `Wears` carriers.
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        authored,
        "the spawned ganger count (each wearing armor via Wears) must equal the authored ganger \
         count (the real setup_battle ran, not a stub)",
    );
}

// === AC5 (sim half) — a FAILED setup emits NO BattleReady, inserts no setup
// resource, and does not panic. ===

#[test]
fn failed_setup_emits_no_ready_and_inserts_no_resource() {
    let (situation, _link) = dangling_link_situation();
    let mut app = headless_app();
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();

    // No BattleReady signalled (fail-closed).
    assert_eq!(
        drain_battle_ready(&mut app),
        0,
        "a failed setup must emit NO BattleReady",
    );
    // setup_battle aborts before any resource insert (validation first).
    assert!(
        app.world().get_resource::<CoverLedger>().is_none(),
        "a failed setup must insert no CoverLedger (it aborts before any resource insert)",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "a failed setup must insert no OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_none(),
        "a failed setup must insert no VerticalLinkGraph",
    );
    // The RNG streams ARE inserted before the setup call (seeding precedes setup), but no
    // grid resources land — the app's gate keys off BattleReady, which is absent.
}

/// Sanity: the dangling-link fixture really yields the typed `DanglingCell` error
/// (so the fail-closed test above is exercising the real error path).
#[test]
fn dangling_link_fixture_yields_the_typed_error() {
    use bevy::ecs::system::RunSystemOnce as _;

    let (situation, link) = dangling_link_situation();
    let gangs = test_gang_registry();
    let registry = weapon_registry();
    let armor = armor_registry();
    let stat_tuning = GangerStatTuning::default();
    // GTW-396: pass `terrain: None` and the fallback floor cost — the dangling-link
    // fixture uses SituationBuilder with no terrain keys, so no registry is needed;
    // the terrain-registry path is exercised by the shipped-situation AC5 test.
    let fallback_floor_cost = crate::tuning::CombatTuning::default().move_costs.open;
    let mut world = World::new();
    let result = world.run_system_once(move |mut commands: Commands| {
        setup_battle(
            &situation,
            BattleRegistries::new(&gangs, &registry, &armor, &stat_tuning, None),
            fallback_floor_cost,
            &mut commands,
        )
    });
    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::InvalidLink(
            InvalidVerticalLink::DanglingCell { link },
        )),
        "the dangling-link fixture must abort setup with the typed DanglingCell error",
    );
}

// === AC6 (sim half) — TeardownBattleRequested after a setup removes the five
// battle-lifetime resources; an injected CombatTuning is untouched. ===

#[test]
fn teardown_removes_battle_resources_and_leaves_tuning() {
    // `headless_app` already inserts CombatTuning (E10.4's persistent Load resource).
    let mut app = headless_app();

    // Set the battle up first.
    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "precondition: setup inserted the RNG streams",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_some(),
        "precondition: setup inserted the OccupancyGrid",
    );
    // GTW-226 AC5 precondition — setup inserted PlayerFaction (present, like
    // BattleInProgress) so the teardown removal is observable.
    assert!(
        app.world().get_resource::<PlayerFaction>().is_some(),
        "precondition: setup inserted the PlayerFaction",
    );

    // Now tear it down.
    app.world_mut().write_message(TeardownBattleRequested);
    app.update();

    assert!(
        app.world().get_resource::<ShotRng>().is_none(),
        "teardown must remove the RNG streams",
    );
    assert!(
        app.world().get_resource::<CoverLedger>().is_none(),
        "teardown must remove the CoverLedger",
    );
    assert!(
        app.world().get_resource::<SurfaceGrid>().is_none(),
        "teardown must remove the SurfaceGrid",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "teardown must remove the OccupancyGrid",
    );
    assert!(
        app.world().get_resource::<VerticalLinkGraph>().is_none(),
        "teardown must remove the VerticalLinkGraph",
    );
    // GTW-226 AC5 — teardown removes PlayerFaction; and BattleInProgress is also
    // absent in the same test, proving the identical battle-active lifetime.
    assert!(
        app.world().get_resource::<PlayerFaction>().is_none(),
        "teardown must remove the PlayerFaction",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "teardown must remove the BattleInProgress witness (identical lifetime to \
         PlayerFaction)",
    );
    // The persistent Load resource is untouched.
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "teardown must NOT remove CombatTuning (E10.4's persistent Load resource)",
    );
}

// === GTW-212 AC1 — BattleInProgress is a public marker Resource with the correct
// lifecycle: ABSENT before setup, PRESENT after a successful setup, ABSENT after
// teardown, and ABSENT after a FAILED setup (no BattleReady → no witness). ===

#[test]
fn battle_in_progress_tracks_the_battle_active_window() {
    let mut app = headless_app();

    // ABSENT before any setup.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "BattleInProgress must be absent before any setup",
    );

    // PRESENT after a successful setup (the small fixture that emits BattleReady).
    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    assert_eq!(
        drain_battle_ready(&mut app),
        1,
        "the fixture setup must succeed (one BattleReady)",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a successful setup must insert the BattleInProgress witness",
    );

    // ABSENT after a teardown.
    app.world_mut().write_message(TeardownBattleRequested);
    app.update();
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "teardown must remove the BattleInProgress witness",
    );
}

#[test]
fn failed_setup_inserts_no_battle_in_progress() {
    let (situation, _link) = dangling_link_situation();
    let mut app = headless_app();
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();

    // A failed setup emits no BattleReady (fail-closed) and inserts no witness.
    assert_eq!(
        drain_battle_ready(&mut app),
        0,
        "a failed setup must emit NO BattleReady",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "a FAILED setup must NOT insert BattleInProgress (the Ok-only witness)",
    );
    // GTW-226 AC4 — fail-closed: PlayerFaction rides the same Ok-only path, so a
    // failed setup inserts no PlayerFaction (mirrors the BattleInProgress absence).
    assert!(
        app.world().get_resource::<PlayerFaction>().is_none(),
        "a FAILED setup must NOT insert PlayerFaction (the Ok-only seed)",
    );
}

// === GTW-466 C3 — ReactionRng shares the BattleInProgress lifetime: PRESENT after
// a successful setup, ABSENT after teardown. ===

/// GTW-466 C3 — [`ReactionRng`] is PRESENT after a successful battle setup and
/// ABSENT after teardown, sharing the [`BattleInProgress`] lifetime exactly.
///
/// The headless app mirrors the real path (the same `BattleSimPlugin` the game
/// uses); no mock, no stub — `setup_battle_on_request` runs on the real
/// [`Commands`] path.
#[test]
fn reaction_rng_present_after_setup_and_absent_after_teardown() {
    let mut app = headless_app();

    // ABSENT before any setup.
    assert!(
        app.world().get_resource::<ReactionRng>().is_none(),
        "GTW-466: ReactionRng must be absent before any battle setup",
    );

    // PRESENT after a successful setup.
    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    assert_eq!(
        drain_battle_ready(&mut app),
        1,
        "the fixture setup must succeed (one BattleReady) — precondition for GTW-466 C3",
    );
    assert!(
        app.world().get_resource::<ReactionRng>().is_some(),
        "GTW-466 C3: ReactionRng must be PRESENT after a successful battle setup",
    );
    // Also verify BattleInProgress is present (the witness we share a lifetime with).
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "GTW-466 C3 precondition: BattleInProgress must be present alongside ReactionRng",
    );

    // ABSENT after teardown.
    app.world_mut().write_message(TeardownBattleRequested);
    app.update();
    assert!(
        app.world().get_resource::<ReactionRng>().is_none(),
        "GTW-466 C3: ReactionRng must be ABSENT after teardown (same lifetime as BattleInProgress)",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "GTW-466 C3: BattleInProgress must be absent alongside ReactionRng after teardown",
    );
}
