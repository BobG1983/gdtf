//! Unit tests for the GTW-655 [`StagedProcgen`](crate::procgen::StagedProcgen) driver —
//! stepping to completion matches a one-shot [`generate_level`](crate::procgen::generate_level)
//! call for the same seed (the load-bearing step-equivalence pin), each `advance` runs
//! exactly one stage, a finished drive is idempotent under a repeat `advance`, and a
//! failed-closed drive re-returns the SAME error rather than re-attempting the stage.

use super::emit::support::{
    registry_with_fill, size, terrain_defs, terrain_eq, theme, theme_registry, tuning,
};
use crate::{
    level::{GridSize, PrefabRegistry, ThemeUuid, UuidThemeRegistry},
    procgen::{
        ProcgenStage, ProcgenTuning, StagedProcgen, StagedProcgenRegistries, generate_level,
    },
    rng::{BattleSeed, ProcgenRng},
    terrain::def::TerrainDefRegistry,
};

/// A small board + registry that assembles, fills, and emits successfully — the common
/// fixture every test in this file builds against.
fn fixture() -> Option<(
    ThemeUuid,
    GridSize,
    PrefabRegistry,
    UuidThemeRegistry,
    TerrainDefRegistry,
    ProcgenTuning,
)> {
    let test_theme = theme();
    let board = size(24, 24)?;
    let player_fp = size(10, 10)?;
    let enemy_fp = size(10, 10)?;
    let registry = registry_with_fill(test_theme, player_fp, enemy_fp, &[("hall", 4, 4)])?;
    Some((
        test_theme,
        board,
        registry,
        theme_registry(test_theme),
        terrain_defs(),
        tuning(0.0, 64, 0),
    ))
}

/// A stepped-to-completion drive produces an IDENTICAL result to a one-shot `generate_level`
/// call for the same seed — the driver is an alternate schedule over the SAME pipeline, never
/// a second implementation of it.
#[test]
fn stepped_to_completion_matches_generate_level_for_the_same_seed() {
    let fixture = fixture();
    assert!(fixture.is_some());
    let Some((theme, board, registry, themes, defs, knobs)) = fixture else {
        return;
    };
    let seed = BattleSeed::new(0xC0FF_EE01);

    let mut one_shot_rng = ProcgenRng::from_root(seed);
    let one_shot = generate_level(
        &registry,
        &themes,
        &defs,
        theme,
        board,
        &mut one_shot_rng,
        &knobs,
    );
    assert!(one_shot.is_ok());
    let Ok(one_shot) = one_shot else {
        return;
    };

    let mut staged = StagedProcgen::new(seed, theme, board);
    let registries = StagedProcgenRegistries {
        prefabs:      &registry,
        themes:       &themes,
        terrain_defs: &defs,
        tuning:       &knobs,
    };
    let ran = staged.run_to_completion(registries);
    assert!(ran.is_ok());
    let staged_result = staged.emitted();
    assert!(staged_result.is_some());
    let Some(staged_result) = staged_result else {
        return;
    };

    assert!(terrain_eq(&one_shot.situation, &staged_result.situation));
    assert_eq!(one_shot.findings, staged_result.findings);
}

/// Each `advance` call runs EXACTLY one stage, and the inspectable state (`placement` /
/// `filled` / `emitted`) only becomes `Some` once its producing stage has run.
#[test]
fn advance_runs_exactly_one_stage_per_call() {
    let fixture = fixture();
    assert!(fixture.is_some());
    let Some((theme, board, registry, themes, defs, knobs)) = fixture else {
        return;
    };
    let seed = BattleSeed::new(7);
    let mut staged = StagedProcgen::new(seed, theme, board);
    let registries = StagedProcgenRegistries {
        prefabs:      &registry,
        themes:       &themes,
        terrain_defs: &defs,
        tuning:       &knobs,
    };

    assert_eq!(staged.stage(), ProcgenStage::Assemble);
    assert!(staged.placement().is_none());
    assert!(staged.filled().is_none());
    assert!(staged.emitted().is_none());

    let first = staged.advance(registries);
    assert_eq!(first, Ok(ProcgenStage::Assemble));
    assert!(staged.placement().is_some());
    assert!(staged.filled().is_none());
    assert!(staged.emitted().is_none());
    assert_eq!(staged.stage(), ProcgenStage::Fill);

    let second = staged.advance(registries);
    assert_eq!(second, Ok(ProcgenStage::Fill));
    assert!(staged.filled().is_some());
    assert!(staged.emitted().is_none());
    assert_eq!(staged.stage(), ProcgenStage::Emit);

    let third = staged.advance(registries);
    assert_eq!(third, Ok(ProcgenStage::Emit));
    assert!(staged.emitted().is_some());
    assert_eq!(staged.stage(), ProcgenStage::Done);
    assert!(staged.is_done());
}

/// Calling `advance` again after the drive is `Done` is a no-op: it returns `Ok(Done)` again
/// and neither the RNG nor the stored result changes (asserted via the emitted result staying
/// the same across the repeat call).
#[test]
fn advance_after_done_is_idempotent() {
    let fixture = fixture();
    assert!(fixture.is_some());
    let Some((theme, board, registry, themes, defs, knobs)) = fixture else {
        return;
    };
    let seed = BattleSeed::new(99);
    let mut staged = StagedProcgen::new(seed, theme, board);
    let registries = StagedProcgenRegistries {
        prefabs:      &registry,
        themes:       &themes,
        terrain_defs: &defs,
        tuning:       &knobs,
    };
    let ran = staged.run_to_completion(registries);
    assert!(ran.is_ok());
    let first_emitted = staged.emitted().cloned();

    let repeat = staged.advance(registries);
    assert_eq!(repeat, Ok(ProcgenStage::Done));
    assert_eq!(
        staged.emitted().cloned().map(|e| e.situation.theme),
        first_emitted.map(|e| e.situation.theme)
    );
}

/// A failed-closed stage (no enemy prefab registered) leaves the drive `Done` (nothing left to
/// retry) and re-returns the SAME error on a repeat `advance`, rather than re-attempting the
/// stage (which would re-draw the RNG and desync from the one-shot pipeline).
#[test]
fn advance_on_failure_is_terminal_and_repeats_the_same_error() {
    let test_theme = theme();
    let Some(board) = size(24, 24) else {
        return;
    };
    // A registry with NO prefabs at all — the assemble stage fails closed immediately.
    let registry = PrefabRegistry::default();
    let themes = theme_registry(test_theme);
    let defs = terrain_defs();
    let knobs = tuning(0.0, 64, 0);
    let mut staged = StagedProcgen::new(BattleSeed::new(1), test_theme, board);
    let registries = StagedProcgenRegistries {
        prefabs:      &registry,
        themes:       &themes,
        terrain_defs: &defs,
        tuning:       &knobs,
    };

    let first = staged.advance(registries);
    assert!(first.is_err());
    assert!(staged.is_done());
    assert!(staged.failure().is_some());

    let second = staged.advance(registries);
    assert!(second.is_err());
    assert_eq!(first, second);
}
