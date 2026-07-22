//! Unit tests for the GTW-655 / GTW-732 [`StagedProcgen`](crate::procgen::StagedProcgen)
//! driver — stepping to completion matches a one-shot
//! [`generate_level`](crate::procgen::generate_level) call for the same seed (the load-bearing
//! step-equivalence pin, now holding BY CONSTRUCTION since one primitive has two drivers), each
//! `advance` lands at most one placement (the per-PREFAB granularity), a finished drive is
//! idempotent under a repeat `advance`, and a failed-closed drive re-returns the SAME error.

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

/// An upper bound on the step count any drive in this file takes — a 24x24 board fills in at
/// most a few dozen prefab placements, so this is a generous guard against a non-terminating
/// drive (never a magic tuning number).
const STEP_BUDGET: usize = 512;

/// A small board + registry that assembles, fills, and emits successfully — the common
/// fixture every test in this file builds against. The tuning here has NO fill floor
/// (`density 0.0`), so the fill places nothing; the per-placement tests override the tuning
/// with a real floor so several fill prefabs land.
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
/// a second implementation of it (clause 4: step-equivalence, now holding by construction).
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

/// Each `advance` call lands AT MOST one placement (the GTW-732 per-prefab granularity): the
/// drive is assemble (exactly two placement steps — player then enemy), then several fill
/// steps (each growing the placement count by one), then the fill-finalize step and the emit
/// step (each adding no placement). Drives until done (a variable step count, since the fill's
/// placement count varies by seed).
#[test]
fn step_advances_exactly_one_placement_per_call() {
    let fixture = fixture();
    assert!(fixture.is_some());
    let Some((theme, board, registry, themes, defs, _)) = fixture else {
        return;
    };
    // A real density floor so several fill prefabs place (unlike the fixture's 0.0 floor).
    let knobs = tuning(0.5, 64, 0);
    let seed = BattleSeed::new(7);
    let mut staged = StagedProcgen::new(seed, theme, board);
    let registries = StagedProcgenRegistries {
        prefabs:      &registry,
        themes:       &themes,
        terrain_defs: &defs,
        tuning:       &knobs,
    };

    assert_eq!(staged.stage(), ProcgenStage::Assemble);
    assert!(staged.placed_footprints().is_empty());

    let mut assemble_steps = 0usize;
    let mut fill_placement_steps = 0usize;
    let mut saw_emit = false;
    let mut prev = staged.placed_footprints().len();
    let mut guard = 0usize;

    while !staged.is_done() {
        guard += 1;
        assert!(guard < STEP_BUDGET, "the staged drive must terminate");
        let advanced = staged.advance(registries);
        assert!(
            advanced.is_ok(),
            "no step must fail for this fixture: {advanced:?}"
        );
        let now = staged.placed_footprints().len();
        assert!(
            now >= prev,
            "placement count must never decrease (got {now} after {prev})"
        );
        assert!(
            now - prev <= 1,
            "a single step must add at most one placement (added {})",
            now - prev,
        );
        match advanced {
            Ok(ProcgenStage::Assemble) => {
                assemble_steps += 1;
                assert_eq!(
                    now - prev,
                    1,
                    "each assemble step places exactly one prefab"
                );
            }
            Ok(ProcgenStage::Fill) => {
                if now - prev == 1 {
                    fill_placement_steps += 1;
                }
            }
            Ok(ProcgenStage::Emit) => {
                saw_emit = true;
                assert_eq!(now - prev, 0, "the emit step adds no placement");
            }
            Ok(ProcgenStage::Done) | Err(_) => {}
        }
        prev = now;
    }

    assert_eq!(
        assemble_steps, 2,
        "assemble is exactly two placement steps (player then enemy)",
    );
    assert!(
        fill_placement_steps >= 1,
        "the 0.5-floor fill must place several fill prefabs; got {fill_placement_steps}",
    );
    assert!(saw_emit, "the drive must run exactly one emit step");
    assert!(staged.is_done());
    assert!(staged.emitted().is_some());
}

/// Driving `advance` one placement at a time to done produces the IDENTICAL emitted level a
/// one-shot `generate_level` call produces for the same seed — exercising the INTERACTIVE
/// per-placement path specifically against the batch loop (clause 4), over a fill-bearing
/// fixture so the fill cursor's per-prefab stepping is under test, not just the empty fill.
#[test]
fn advance_per_placement_matches_generate_level_for_the_same_seed() {
    let fixture = fixture();
    assert!(fixture.is_some());
    let Some((theme, board, registry, themes, defs, _)) = fixture else {
        return;
    };
    let knobs = tuning(0.5, 64, 0);
    let seed = BattleSeed::new(0x00AB_CD11);

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
    let mut guard = 0usize;
    while !staged.is_done() {
        guard += 1;
        assert!(guard < STEP_BUDGET, "the staged drive must terminate");
        let advanced = staged.advance(registries);
        assert!(advanced.is_ok(), "no step must fail: {advanced:?}");
    }
    let staged_result = staged.emitted();
    assert!(staged_result.is_some());
    let Some(staged_result) = staged_result else {
        return;
    };
    assert!(
        terrain_eq(&one_shot.situation, &staged_result.situation),
        "advance-per-placement must equal generate_level for the same seed (step-equivalence)",
    );
    assert_eq!(one_shot.findings, staged_result.findings);
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

/// A failed-closed step (no prefab registered) leaves the drive `Done` (nothing left to
/// retry) and re-returns the SAME error on a repeat `advance`, rather than re-attempting the
/// step (which would re-draw the RNG and desync from the one-shot pipeline).
#[test]
fn advance_on_failure_is_terminal_and_repeats_the_same_error() {
    let test_theme = theme();
    let Some(board) = size(24, 24) else {
        return;
    };
    // A registry with NO prefabs at all — the first (player-placement) step fails closed.
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
