//! The LOAD-BEARING step-equivalence tests: driving the staged pipeline one `Next` press at a
//! time to completion (bypassing egui entirely — the closure never runs headlessly, bevy-traps
//! #8) must produce the IDENTICAL result the normal to-completion path produces, and the
//! schematic's per-placement data (`placed_footprints`) must grow one placement per step over
//! the real shipped content (GTW-732).

use gdtf_app::test_support::{BattleScapeState, LoadedSituation};
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::{ProcgenTuning, StagedProcgen, StagedProcgenRegistries},
    rng::BattleSeed,
    terrain::def::TerrainDefRegistry,
};
use gdtf_test_utils::advance_until;

use super::harness::{
    BUDGET, FIXED_SEED, app_engaged_in_generation, app_ready_for_battle, battlescape_state,
    deployed_ganger_count, drive_into_battle_running, drive_stepper_to_done, terrain_fingerprint,
};

/// The load-bearing step-equivalence test (clause 4): engaging the stepper and driving it ONE
/// PREFAB AT A TIME to completion (via the SAME latch the egui panel writes to) for the SAME
/// seed reaches `BattleRunning` with the IDENTICAL terrain fingerprint the normal path produces
/// — a VARIABLE step count now that Fill places one prefab per `Next`.
#[test]
fn stepper_engaged_path_matches_the_normal_fingerprint() {
    let expected = {
        let mut app = app_ready_for_battle(FIXED_SEED);
        drive_into_battle_running(&mut app);
        terrain_fingerprint(&app)
    };

    let mut app = app_engaged_in_generation(FIXED_SEED);
    // Drive every placement (assemble x2, then each fill prefab, then finalize + emit) one Next
    // press at a time until the drive completes — bypassing egui entirely.
    drive_stepper_to_done(&mut app);

    let reached_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_running,
        "the stepper-engaged path must reach BattleRunning once its staged drive completes; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let actual = terrain_fingerprint(&app);
    assert_eq!(
        actual, expected,
        "the stepper-engaged, stepped-to-completion drive must produce the SAME terrain \
         fingerprint as the normal to-completion path for the same seed",
    );
}

/// GTW-765 regression (must not regress under GTW-732): the stepper-engaged finish must DEPLOY
/// the authored roster onto the generated map, not just its terrain. Driving the staged pipeline
/// to `BattleRunning` (the SAME per-prefab latch the egui panel writes) must leave the SAME
/// non-zero count of deployed ganger entities the normal to-completion path deploys for the same
/// seed — the pre-fix terrain-only `finish_stepper_drive` left ZERO gangers, which
/// `terrain_fingerprint` (terrain entities only) could not catch.
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

    let reached_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_running,
        "the stepper-engaged path must reach BattleRunning once its staged drive completes; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    let actual = deployed_ganger_count(&mut app);
    assert_eq!(
        actual, expected,
        "the stepper-engaged finish must DEPLOY the same roster the normal path does (GTW-765): \
         the pre-fix terrain-only finish left {actual} deployed gangers, expected {expected}",
    );
}

/// The schematic's per-placement data (GTW-732): driving a FRESH [`StagedProcgen`] one step at a
/// time over the REAL shipped content grows [`placed_footprints`](StagedProcgen::placed_footprints)
/// by AT MOST one per step (exactly one per placement; zero on the finalize + emit steps), and
/// [`emitted`](StagedProcgen::emitted) becomes `Some` once done. This is the data the egui
/// schematic reads to draw the map assembling piece by piece.
///
/// Drives a FRESH `StagedProcgen` directly in the test body (bevy-traps #7 carve-out) rather
/// than through `PendingStepCommand` + `app.update()`: `finish_stepper_drive` removes the
/// `StagedProcgen` resource in the SAME `Update` pass the emit step completes in, too narrow a
/// window to read the growing footprint list back out of the world afterward. Reads the SAME
/// registries + authored theme/grid-size `engage_stepper` would, off the real `Load`-populated
/// world, so this is the real pipeline over shipped content, not a synthetic fixture.
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
    let mut driver =
        StagedProcgen::new(BattleSeed::new(FIXED_SEED), loaded.theme, loaded.grid_size);

    assert!(
        driver.placed_footprints().is_empty(),
        "a fresh driver has landed no placements yet",
    );

    let mut prev = 0usize;
    let mut guard = 0u32;
    while !driver.is_done() {
        guard += 1;
        assert!(guard < BUDGET, "the staged drive must terminate");
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
