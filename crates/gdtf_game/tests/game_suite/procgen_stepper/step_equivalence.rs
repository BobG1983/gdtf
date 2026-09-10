use cobalt_test_utils::advance_until;
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::{ProcgenTuning, StagedProcgen, StagedProcgenRegistries},
    rng::BattleSeed,
    terrain::def::TerrainDefRegistry,
};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{BattleScapeState, PendingStepCommand, StepCommand};

use super::harness::{
    FIXED_SEED, app_engaged_in_generation, app_ready_for_battle, battlescape_state,
    deployed_ganger_count, drive_into_battle_running, drive_stepper_to_done, terrain_fingerprint,
};

/// Runaway backstop for the pure staged driver — deterministic compute, no clock.
const DRIVE_GUARD: u32 = 512;

#[test]
fn stepper_engaged_path_matches_the_normal_fingerprint() {
    let expected = {
        let mut app = app_ready_for_battle(FIXED_SEED);
        drive_into_battle_running(&mut app);
        terrain_fingerprint(&app)
    };

    let mut app = app_engaged_in_generation(FIXED_SEED);
    drive_stepper_to_done(&mut app);

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });

    let actual = terrain_fingerprint(&app);
    assert_eq!(
        actual, expected,
        "the stepper-engaged, stepped-to-completion drive must produce the SAME terrain \
         fingerprint as the normal to-completion path for the same seed",
    );
}

#[test]
fn stepper_engaged_path_deploys_the_same_roster() {
    let expected = {
        let mut app = app_ready_for_battle(FIXED_SEED);
        drive_into_battle_running(&mut app);
        deployed_ganger_count(&mut app)
    };
    assert!(
        expected > 0,
        "the normal path must deploy a non-empty roster for the fixed seed (guarding the test \
         itself); got {expected} deployed gangers",
    );

    let mut app = app_engaged_in_generation(FIXED_SEED);
    drive_stepper_to_done(&mut app);

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });

    let actual = deployed_ganger_count(&mut app);
    assert_eq!(
        actual, expected,
        "the stepper-engaged finish must DEPLOY the same roster the normal path does: \
         the pre-fix terrain-only finish left {actual} deployed gangers, expected {expected}",
    );
}

#[test]
fn a_skipped_generation_deploys_the_same_roster() {
    let expected = {
        let mut app = app_ready_for_battle(FIXED_SEED);
        drive_into_battle_running(&mut app);
        deployed_ganger_count(&mut app)
    };
    assert!(
        expected > 0,
        "the normal path must deploy a non-empty roster for the fixed seed (guarding the test \
         itself); got {expected} deployed gangers",
    );

    let mut app = app_engaged_in_generation(FIXED_SEED);
    let pending = app.world_mut().get_resource_mut::<PendingStepCommand>();
    assert!(
        pending.is_some(),
        "PendingStepCommand must exist while the stepper is engaged in Generation",
    );
    if let Some(mut pending) = pending {
        pending.request(StepCommand::Skip);
    }
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });

    let actual = deployed_ganger_count(&mut app);
    assert_eq!(
        actual, expected,
        "Skip runs every remaining stage in one update, and that finish must DEPLOY the same \
         roster the normal path does; got {actual} deployed gangers, expected {expected}",
    );
}

#[test]
fn placed_footprints_grow_one_per_step_over_real_content() {
    let app = app_ready_for_battle(FIXED_SEED);
    let world = app.world();
    let resources = (
        world.get_resource::<PrefabRegistry>(),
        world.get_resource::<UuidThemeRegistry>(),
        world.get_resource::<TerrainDefRegistry>(),
        world.get_resource::<LoadedSituation>(),
    );
    assert!(
        matches!(resources, (Some(_), Some(_), Some(_), Some(_))),
        "the real Load flow must populate every registry + the authored situation",
    );
    let (Some(prefabs), Some(themes), Some(terrain_defs), Some(loaded)) = resources else {
        return;
    };
    let default_tuning = ProcgenTuning::default();
    let tuning = world
        .get_resource::<ProcgenTuning>()
        .unwrap_or(&default_tuning);
    let registries = StagedProcgenRegistries {
        prefabs,
        themes,
        terrain_defs,
        tuning,
    };
    let mut driver = StagedProcgen::new(
        BattleSeed::new(FIXED_SEED),
        loaded.map.theme,
        loaded.map.grid_size,
    );

    assert!(
        driver.placed_footprints().is_empty(),
        "a fresh driver has landed no placements yet",
    );

    let mut prev = 0usize;
    let mut guard = 0u32;
    while !driver.is_done() {
        guard += 1;
        assert!(guard < DRIVE_GUARD, "the staged drive must terminate");
        let advanced = driver.advance(registries);
        assert!(
            advanced.is_ok(),
            "no step must fail over the shipped registries for the fixed seed: {advanced:?}",
        );
        let now = driver.placed_footprints().len();
        assert!(
            now >= prev && now - prev <= 1,
            "each step lands AT MOST one placement (was {prev}, now {now})",
        );
        prev = now;
    }

    assert!(
        driver.emitted().is_some(),
        "the drive must emit a level once done",
    );
    assert!(
        prev >= 2,
        "the assemble stage alone lands two placements (player + enemy); the schematic saw {prev}",
    );
}
