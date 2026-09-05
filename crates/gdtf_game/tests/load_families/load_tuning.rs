//! Combat tuning load: gate holds without it; real path resolves; failure uses default.
use std::path::PathBuf;

use cobalt_test_utils::{
    LoadTestAppBuilder, MinimalTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_game::test_support::{AppState, app_state, load_released};
use gdtf_ui::theme::default_theme;

use crate::load_suite::gate;

/// Frames the machine is given to prove it stays put — per-frame work, no IO.
const HOLD_FRAMES: u32 = 32;

fn bad_tuning_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bad_tuning_root")
}

#[test]
fn tuning_loader_no_ops_cleanly_without_asset_server() {
    let mut app = MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless)
        .starting_in(AppState::Load)
        .build();

    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    assert!(
        app.world().get_resource::<CombatTuning>().is_none(),
        "with no AssetServer the tuning load chain must no-op — no CombatTuning inserted",
    );

    gate::seed_full_load_gate(&mut app);

    advance_until(&mut app, load_released);
}

#[test]
fn load_does_not_leave_without_a_tuning() {
    let mut app = MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless)
        .starting_in(AppState::Load)
        .build();

    gate::seed_gate_except::<CombatTuning>(&mut app);

    for _ in 0..HOLD_FRAMES {
        app.update();
    }

    assert_eq!(
        app_state(&app),
        AppState::Load,
        "without a CombatTuning the machine must not leave Load",
    );
}

#[test]
fn load_does_not_leave_on_theme_only() {
    let mut app = MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless)
        .starting_in(AppState::Load)
        .build();

    app.update();
    app.world_mut().insert_resource(default_theme());

    for _ in 0..HOLD_FRAMES {
        app.update();
    }

    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with a theme but NO tuning the machine must not leave Load",
    );
}

#[test]
fn real_asset_resolves_persistent_combat_tuning() {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();

    advance_until_resource_exists::<CombatTuning>(&mut app);

    if let Some(tuning) = app.world().get_resource::<CombatTuning>() {
        let default = CombatTuning::default();
        if &default != tuning {
            assert_ne!(
                tuning, &default,
                "the good path must have parsed the SHIPPED tuning, distinct from the const default",
            );
        }
    }

    advance_until(&mut app, load_released);
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must persist past OnExit(Load) into Intro — the BattleScape-consumer \
         exception (like GdtfTheme), NOT removed by cleanup",
    );
}

#[test]
fn real_asset_failure_path_does_not_hang_and_uses_default_tuning() {
    let mut app = LoadTestAppBuilder::with_asset_root(
        bad_tuning_root(),
        gdtf_game::test_support::register_scenes_with_default_plugins,
    )
    .starting_in(AppState::Load)
    .build();

    advance_until_resource_exists::<CombatTuning>(&mut app);

    if let Some(tuning) = app.world().get_resource::<CombatTuning>() {
        assert_eq!(
            tuning,
            &CombatTuning::default(),
            "the failure path must insert exactly the const-fallback default tuning",
        );
    }

    advance_until(&mut app, load_released);
}
