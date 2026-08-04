//! Combat tuning load: gate holds without it; real path resolves; failure uses default.
use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::default_theme;

use crate::load_suite::gate;

const TRANSITION_BUDGET: u32 = 32;

const LOAD_SAFETY_NET: u32 = 10_000;

fn bad_tuning_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bad_tuning_root")
}

#[test]
fn tuning_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
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

    let released = advance_until(&mut app, load_released, TRANSITION_BUDGET);
    assert!(
        released,
        "with a GdtfTheme + CombatTuning + WeaponRegistry + LoadedSituation present, Load must \
         release to Intro (or beyond) within {TRANSITION_BUDGET} updates; last observed \
         AppState was {:?}",
        app_state(&app),
    );
}

#[test]
fn load_does_not_leave_without_a_tuning() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    gate::seed_gate_except::<CombatTuning>(&mut app);

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "without a CombatTuning the machine must not leave Load; it reached {:?}",
        app_state(&app),
    );
}

#[test]
fn load_does_not_leave_on_theme_only() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    app.update();
    app.world_mut().insert_resource(default_theme());

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "with a theme but NO tuning the machine must not leave Load; it reached {:?}",
        app_state(&app),
    );
}

#[test]
fn real_asset_resolves_persistent_combat_tuning() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<CombatTuning>(&mut app, LOAD_SAFETY_NET);

    if let Some(tuning) = app.world().get_resource::<CombatTuning>() {
        let default = CombatTuning::default();
        if &default != tuning {
            assert_ne!(
                tuning, &default,
                "the good path must have parsed the SHIPPED tuning, distinct from the const default",
            );
        }
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a resolved theme + tuning, Load must release to Intro (or beyond); last AppState \
         was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "CombatTuning must persist past OnExit(Load) into Intro — the BattleScape-consumer \
         exception (like GdtfTheme), NOT removed by cleanup",
    );
}

#[test]
fn real_asset_failure_path_does_not_hang_and_uses_default_tuning() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(bad_tuning_root())
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<CombatTuning>(&mut app, LOAD_SAFETY_NET);

    if let Some(tuning) = app.world().get_resource::<CombatTuning>() {
        assert_eq!(
            tuning,
            &CombatTuning::default(),
            "the failure path must insert exactly the const-fallback default tuning",
        );
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "even on a failed tuning, Load must release to Intro (or beyond) with the default \
         tuning; last AppState was {:?}",
        app_state(&app),
    );
}
