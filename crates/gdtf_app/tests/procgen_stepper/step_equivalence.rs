//! The LOAD-BEARING step-equivalence tests: driving the staged pipeline one `Next` press at a
//! time (bypassing egui entirely — the closure never runs headlessly, bevy-traps #8) must
//! produce the IDENTICAL result the normal to-completion path produces, and the panel's pure
//! summary formatter must name the REAL driver state at every stage transition.

use gdtf_app::test_support::{
    BattleScapeState, LoadedSituation, PendingStepCommand, StepCommand, stage_summary,
};
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::{ProcgenTuning, StagedProcgen, StagedProcgenRegistries},
    rng::BattleSeed,
    terrain::def::TerrainDefRegistry,
};
use gdtf_test_utils::advance_until;

use super::harness::{
    BUDGET, FIXED_SEED, app_engaged_in_generation, app_ready_for_battle, battlescape_state,
    drive_into_battle_running, terrain_fingerprint,
};

/// The load-bearing step-equivalence test (C5a): engaging the stepper and driving it ONE STAGE
/// AT A TIME (via the SAME latch the egui panel writes to) for the SAME seed reaches
/// `BattleRunning` with the IDENTICAL terrain fingerprint the normal path produces.
#[test]
fn stepper_engaged_path_matches_the_normal_fingerprint() {
    let expected = {
        let mut app = app_ready_for_battle(FIXED_SEED);
        drive_into_battle_running(&mut app);
        terrain_fingerprint(&app)
    };

    let mut app = app_engaged_in_generation(FIXED_SEED);

    // Drive every stage (assemble -> fill -> emit) one Next press at a time — bypassing egui
    // entirely (the closure never runs headlessly): each request+update pair mirrors one press.
    for _ in 0..3 {
        let pending = app.world_mut().get_resource_mut::<PendingStepCommand>();
        assert!(
            pending.is_some(),
            "PendingStepCommand must exist while the stepper is engaged in Generation",
        );
        if let Some(mut pending) = pending {
            pending.request(StepCommand::Next);
        }
        app.update();
    }

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

/// [`stage_summary`] (the panel's pure formatter) names the REAL driver state the shipped
/// registries produce at every stage transition — not just the two branches reachable with an
/// empty/fresh driver (`dev::procgen_stepper::summary::test` unit-checks those).
///
/// Drives a FRESH [`StagedProcgen`] directly in the test body (bevy-traps #7 carve-out) rather
/// than through `PendingStepCommand` + `app.update()`: `finish_stepper_drive` removes the
/// `StagedProcgen` resource in the SAME `Update` pass the emit stage completes in, too narrow a
/// window to read the "Emitted" summary back out of the world afterward. Reads the SAME
/// registries + authored theme/grid-size `engage_stepper` would, off the real `Load`-populated
/// world, so this is the real pipeline over shipped content, not a synthetic fixture.
#[test]
fn stage_summary_reflects_the_real_driver_at_each_stage() {
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
        stage_summary(&driver).contains("Not started"),
        "a fresh driver must summarize as not-yet-started",
    );

    let assembled = driver.advance(registries);
    assert!(
        assembled.is_ok(),
        "the real shipped registries must assemble successfully for the fixed seed; got \
         {assembled:?}",
    );
    assert!(
        stage_summary(&driver).contains("Assembled:"),
        "after the assemble stage, the summary must name the real placement",
    );

    let filled = driver.advance(registries);
    assert!(
        filled.is_ok(),
        "the real shipped registries must fill successfully for the fixed seed; got {filled:?}",
    );
    assert!(
        stage_summary(&driver).contains("Filled:"),
        "after the fill stage, the summary must name the real fill counts",
    );

    let emitted = driver.advance(registries);
    assert!(
        emitted.is_ok(),
        "the real shipped registries must emit successfully for the fixed seed; got {emitted:?}",
    );
    assert!(
        driver.is_done(),
        "the drive must be done after the emit stage"
    );
    assert!(
        stage_summary(&driver).contains("Emitted:"),
        "after the emit stage, the summary must name the real terrain counts",
    );
}
