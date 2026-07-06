//! GTW-498 config inputs + Generate: seeding, rebuild, determinism, validation (C1-C6).

use gdtf_app::test_support::{
    GenerateButton, HeightField, LevelsField, ProcgenViz, SizeStatusText, WidthField,
};

use super::{form::*, harness::*};

/// C6: the INITIAL state (before any Generate) seeds the config from the loaded situation +
/// default seed — so the panel opens on the same inputs the prior GTW-434 build used.
///
/// Pin: a wrong initial theme / size (e.g. defaulting instead of reading the situation) reddens.
#[test]
fn initial_config_seeds_from_situation() {
    let app = viz_app();
    let Some(config) = config(&app) else {
        unreachable!("the VizConfig must be inserted OnEnter (C6)")
    };
    assert_eq!(
        config.theme(),
        viz_theme(),
        "the initial config theme must be the loaded situation's theme (C6)",
    );
    let Ok(size) = config.grid_size() else {
        unreachable!("the initial size (from the 30x30x4 situation) is valid (C6)")
    };
    assert_eq!(
        (*size.width(), *size.height(), *size.levels()),
        (30, 30, 4),
        "the initial config size must be the loaded situation's grid-size (C6)",
    );
    // The injected BattleSeed override seeds the config seed (the reproducible-harness path).
    assert_eq!(
        *config.seed(),
        TEST_SEED,
        "the initial config seed must be the injected BattleSeed override (C3/C6)",
    );
}

/// C1/C2/C5: editing theme + size, then pressing Generate, regenerates the model from those
/// inputs — the regenerated board reflects the chosen size, and a DIFFERENT theme is honoured.
///
/// Pin: an input that does not drive a regenerate (the feature-completeness rule), or a Generate
/// that ignores the config, reddens the board-dimension assertion.
#[test]
fn generate_rebuilds_from_theme_and_size() {
    let mut app = viz_app();
    let before = board_dims(&app);
    assert_eq!(before, (30, 30), "precondition: the initial board is 30x30");

    // Choose a different theme + a different board (still large enough to fit the 12x12 player +
    // enemy fragments at opposite corners) on the real widget-message path.
    select_theme(&mut app, other_theme());
    commit_u8_field::<WidthField>(&mut app, 40);
    commit_u8_field::<HeightField>(&mut app, 44);
    commit_u8_field::<LevelsField>(&mut app, 2);

    // The selections updated the config but NOT yet the rendered level (apply-on-Generate, C5).
    assert_eq!(
        config_theme(&app),
        Some(other_theme()),
        "selecting a theme updates the config (C1)",
    );
    assert_eq!(
        board_dims(&app),
        before,
        "selecting inputs must NOT regenerate until Generate is pressed (C5)",
    );

    press_button::<GenerateButton>(&mut app);

    assert_eq!(
        board_dims(&app),
        (40, 44),
        "Generate must rebuild the model from the chosen size (C2/C5)",
    );
    // The model is non-empty (the chosen theme keyed real player + enemy prefabs — C1).
    assert!(
        total(&app) >= 2,
        "the regenerated level under the chosen theme must place player + enemy (C1)",
    );
    // Generate resets the reveal to zero (C5).
    assert_eq!(
        revealed(&app),
        0,
        "Generate must reset the reveal to zero (C5)"
    );
}

/// C3/C5: regeneration is DETERMINISTIC per seed — two DIFFERENT seeds yield DIFFERENT placements
/// while the SAME seed reproduces the placement.
///
/// Pin: a seed that does not drive the RNG (a constant placement), or a non-deterministic build,
/// reddens. Asserts a structural fact (the ordered quad rectangles differ / match), never a
/// tunable magnitude.
#[test]
fn generate_is_deterministic_per_seed() {
    /// The ordered placement-RECTANGLE fingerprint of the current model — the player + enemy
    /// quad rectangles, which the seed's anchor draw moves (a structural signature, not a
    /// tunable magnitude).
    fn fingerprint(app: &bevy::app::App) -> Vec<(u32, u32, u32, u32)> {
        let n = total(app);
        (0..n)
            .filter_map(|i| {
                app.world()
                    .get_resource::<ProcgenViz>()
                    .and_then(|model| model.quad_rect_at(i))
            })
            .collect()
    }

    let mut app = viz_app();

    // Generate across a SPREAD of seeds. The player anchor is an RNG draw, so the placement
    // moves with the seed — across several seeds NOT ALL placements can be identical (proving
    // the seed drives the placement, C3), without assuming any single pair differs.
    let seeds: [u64; 6] = [0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666];
    let mut prints: Vec<Vec<(u32, u32, u32, u32)>> = Vec::new();
    for seed in seeds {
        commit_seed(&mut app, seed);
        press_button::<GenerateButton>(&mut app);
        prints.push(fingerprint(&app));
    }
    assert!(
        prints.iter().all(|p| !p.is_empty()),
        "precondition: every regenerated level is non-empty",
    );
    let distinct: std::collections::HashSet<_> = prints.iter().cloned().collect();
    assert!(
        distinct.len() > 1,
        "different seeds must yield different placements — the seed drives procgen (C3); all \
         {} seeds produced the SAME placement {:?}",
        seeds.len(),
        prints.first(),
    );

    // The SAME seed reproduces the placement (re-generate the first seed, expect its print).
    commit_seed(&mut app, seeds[0]);
    press_button::<GenerateButton>(&mut app);
    assert_eq!(
        fingerprint(&app),
        prints[0],
        "the SAME seed must reproduce the placement (C3)",
    );
}

/// C2: an INVALID size combo is rejected without panic — the Generate button is DISABLED, the
/// status readout shows the error, and a Generate press leaves the model unchanged (so an
/// invalid size never regenerates).
///
/// Drives a real `NumericFieldCommitted<u8>` of ZERO for the width axis (a zero axis is what
/// `GridSize::new` rejects), then asserts the disable + skip path on the REAL listeners. Pin: a
/// missing disable, a status that does not surface the error, or a Generate that rebuilds anyway
/// reddens.
#[test]
fn invalid_size_is_rejected_without_panic() {
    let mut app = viz_app();
    let before = board_dims(&app);
    let before_total = total(&app);

    // A zero WIDTH makes the combo invalid (GridSize::new rejects a zero axis) WITHOUT panicking.
    commit_u8_field::<WidthField>(&mut app, 0);

    assert!(
        !config_size_valid(&app),
        "a zero-width combo must be rejected by GridSize::new — surfaced as Err, never a panic (C2)",
    );
    // The status readout surfaces the rejection (not `OK`).
    let Some(status) = single_with::<SizeStatusText>(&mut app) else {
        unreachable!("the panel spawns exactly one status text")
    };
    let status_text = app
        .world()
        .get::<bevy::prelude::Text>(status)
        .map(|t| t.0.clone())
        .unwrap_or_default();
    assert!(
        !status_text.starts_with("OK"),
        "the status readout must surface the invalid size (not OK); was {status_text:?}",
    );
    // Generate is DISABLED for the invalid combo.
    assert!(
        generate_disabled(&mut app),
        "Generate must be DISABLED for an invalid size combo (C2)",
    );
    // A Generate press leaves the model unchanged (the disabled button + the in-system re-check
    // both guard it — an invalid size NEVER regenerates).
    press_button::<GenerateButton>(&mut app);
    assert_eq!(
        board_dims(&app),
        before,
        "an invalid size must leave the model board unchanged (C2)",
    );
    assert_eq!(
        total(&app),
        before_total,
        "an invalid size must leave the model quads unchanged (C2)",
    );

    // Restoring a valid width re-enables Generate (the disable is not sticky).
    commit_u8_field::<WidthField>(&mut app, 30);
    assert!(
        config_size_valid(&app),
        "a restored valid width makes the combo valid again (C2)",
    );
    assert!(
        !generate_disabled(&mut app),
        "Generate must RE-ENABLE once the size combo is valid again (C2)",
    );
}
