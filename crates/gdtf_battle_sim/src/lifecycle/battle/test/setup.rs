use bevy::prelude::{Commands, World};

use super::support::*;
use crate::rng::ReactionRng;


#[test]
fn setup_request_seeds_rng_inserts_resources_and_signals_ready() {
    let mut app = headless_app();
    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();

    assert!(
        app.world().get_resource::<ShotRng>().is_some(),
        "the setup must insert ShotRng",
    );
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
    assert_eq!(
        drain_battle_ready(&mut app),
        1,
        "a successful setup must emit exactly one BattleReady",
    );
}

#[test]
fn setup_threads_the_message_seed_through_rng_streams() {
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


#[test]
fn setup_runs_setup_battle_on_the_real_commands_path() {
    let situation = two_ganger_situation();
    let authored = situation.gangers.len();
    let mut app = headless_app();
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();
    app.update();

    let world = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        authored,
        "the spawned ganger count (each wearing armor via Wears) must equal the authored ganger \
         count (the real setup_battle ran, not a stub)",
    );
}


#[test]
fn failed_setup_emits_no_ready_and_inserts_no_resource() {
    let (situation, _link) = dangling_link_situation();
    let mut app = headless_app();
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();

    assert_eq!(
        drain_battle_ready(&mut app),
        0,
        "a failed setup must emit NO BattleReady",
    );
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
}

#[test]
fn dangling_link_fixture_yields_the_typed_error() {
    use bevy::ecs::system::RunSystemOnce as _;

    let (situation, link) = dangling_link_situation();
    let gangs = test_gang_registry();
    let registry = weapon_registry();
    let melee = melee_weapon_registry();
    let armor = armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let fallback_floor_cost = crate::tuning::CombatTuning::default().move_costs.open;
    let mut world = World::new();
    let result = world.run_system_once(move |mut commands: Commands| {
        setup_battle(
            &situation,
            BattleRegistries::new(&gangs, &registry, &melee, &armor, &stat_tuning, None),
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


#[test]
fn teardown_removes_battle_resources_and_leaves_tuning() {
    let mut app = headless_app();

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
    assert!(
        app.world().get_resource::<PlayerFaction>().is_some(),
        "precondition: setup inserted the PlayerFaction",
    );

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
    assert!(
        app.world().get_resource::<PlayerFaction>().is_none(),
        "teardown must remove the PlayerFaction",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "teardown must remove the BattleInProgress witness (identical lifetime to \
         PlayerFaction)",
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "teardown must NOT remove CombatTuning (E10.4's persistent Load resource)",
    );
}


#[test]
fn battle_in_progress_tracks_the_battle_active_window() {
    let mut app = headless_app();

    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "BattleInProgress must be absent before any setup",
    );

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

    assert_eq!(
        drain_battle_ready(&mut app),
        0,
        "a failed setup must emit NO BattleReady",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "a FAILED setup must NOT insert BattleInProgress (the Ok-only witness)",
    );
    assert!(
        app.world().get_resource::<PlayerFaction>().is_none(),
        "a FAILED setup must NOT insert PlayerFaction (the Ok-only seed)",
    );
}


#[test]
fn reaction_rng_present_after_setup_and_absent_after_teardown() {
    let mut app = headless_app();

    assert!(
        app.world().get_resource::<ReactionRng>().is_none(),
        "GTW-466: ReactionRng must be absent before any battle setup",
    );

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
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "GTW-466 C3 precondition: BattleInProgress must be present alongside ReactionRng",
    );

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
