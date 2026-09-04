//! An authored ALL-ZERO `body_part_weights` table is INVALID DATA and must fall back.
use std::path::PathBuf;

use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_game::test_support::{AppState, load_released};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use gdtf_ui::theme::GdtfTheme;

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

    advance_until_resource_exists::<CombatTuning>(&mut app);

    let tuning = app.world().get_resource::<CombatTuning>();
    assert_eq!(
        tuning,
        Some(&CombatTuning::default()),
        "an all-zero body_part_weights table must be REJECTED at load — the resolve \
         must insert exactly the const-fallback default tuning, never the authored \
         all-zero table",
    );

    advance_until(&mut app, |app| {
        app.world().get_resource::<GdtfTheme>().is_some()
    });

    advance_until(&mut app, load_released);
}
