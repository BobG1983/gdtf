//! An authored ALL-ZERO `body_part_weights` table is INVALID DATA and must fall back.
use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use gdtf_ui::theme::GdtfTheme;

const LOAD_SAFETY_NET: u32 = 10_000;

fn all_zero_weights_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("all_zero_weights_root")
}

#[test]
fn all_zero_weights_tuning_is_rejected_and_falls_back_to_default() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(all_zero_weights_root())
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<CombatTuning>(&mut app, LOAD_SAFETY_NET);

    let tuning = app.world().get_resource::<CombatTuning>();
    assert_eq!(
        tuning,
        Some(&CombatTuning::default()),
        "an all-zero body_part_weights table must be REJECTED at load — the resolve \
         must insert exactly the const-fallback default tuning, never the authored \
         all-zero table",
    );

    let theme_loaded = advance_until(
        &mut app,
        |app| app.world().get_resource::<GdtfTheme>().is_some(),
        LOAD_SAFETY_NET,
    );
    assert!(
        theme_loaded,
        "the valid sibling core_tuning/ui_theme.tuning.ron must still resolve — the \
         all-zero-weights rejection must not take down the load pass",
    );

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "even with the all-zero-weights tuning rejected, Load must release to Intro \
         (or beyond) with the default tuning; last AppState was {:?}",
        app_state(&app),
    );
}
