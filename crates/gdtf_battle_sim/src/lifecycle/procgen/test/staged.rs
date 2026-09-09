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

const STEP_BUDGET: usize = 512;

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

    assert!(terrain_eq(&one_shot.map, &staged_result.map));
    assert_eq!(one_shot.findings, staged_result.findings);
}

#[test]
fn step_advances_exactly_one_placement_per_call() {
    let fixture = fixture();
    assert!(fixture.is_some());
    let Some((theme, board, registry, themes, defs, _)) = fixture else {
        return;
    };
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
    assert_eq!(staged.placed_footprints(), []);

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
        terrain_eq(&one_shot.map, &staged_result.map),
        "advance-per-placement must equal generate_level for the same seed (step-equivalence)",
    );
    assert_eq!(one_shot.findings, staged_result.findings);
}

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
        staged.emitted().cloned().map(|e| e.map.theme),
        first_emitted.map(|e| e.map.theme)
    );
}

#[test]
fn advance_on_failure_is_terminal_and_repeats_the_same_error() {
    let test_theme = theme();
    let Some(board) = size(24, 24) else {
        return;
    };
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
