//! GTW-205 (E10.3): `AppState::Load` loads the authored `Situation` `.ron`
#[path = "load_suite/gate.rs"]
mod gate;

use std::path::PathBuf;

use gdtf_app::test_support::{AppState, LoadedSituation, app_state, load_released};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};
use gdtf_ui::theme::GdtfTheme;

const TRANSITION_BUDGET: u32 = 32;

const LOAD_SAFETY_NET: u32 = 10_000;

#[test]
fn situation_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
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

    assert!(
        app.world().get_resource::<LoadedSituation>().is_some(),
        "the seeded LoadedSituation must be the one that cleared the gate (no AssetServer resolve)",
    );
}

#[test]
fn real_asset_resolves_persistent_loaded_situation() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<LoadedSituation>(&mut app, LOAD_SAFETY_NET);

    if let Some(loaded) = app.world().get_resource::<LoadedSituation>() {
        assert!(
            !loaded.rosters.is_empty(),
            "the resolved LoadedSituation must carry the authored (non-empty) roster members",
        );
    }

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        LOAD_SAFETY_NET,
    );
    assert!(
        left_load,
        "Load must advance once a theme is present; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<LoadedSituation>().is_some(),
        "LoadedSituation must persist past OnExit(Load) — the Generation-consumer exception",
    );
}

#[test]
fn real_asset_gate_waits_for_the_real_situation() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a real AssetServer, Load must release to Intro (or beyond) once theme + situation \
         resolve; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the gated transition fired — a GdtfTheme is present",
    );
    let loaded = app.world().get_resource::<LoadedSituation>();
    assert!(
        loaded.is_some(),
        "a LoadedSituation must be present at Intro (the gate waited for it)",
    );
    if let Some(loaded) = loaded {
        assert!(
            !loaded.rosters.is_empty(),
            "the situation that cleared the gate must be the real (non-empty) skirmish, not the \
             empty default — Load waited for the real situation (GTW-744: distinguished by its \
             non-empty roster, since the shipped skirmish authors `rosters`, not placed `gangers`)",
        );
    }
}

#[test]
fn load_does_not_leave_without_a_situation() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    gate::seed_gate_except::<LoadedSituation>(&mut app);

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );
    assert!(
        !left_load,
        "Load must NOT leave while the LoadedSituation is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no LoadedSituation present, the machine stays in Load (a battle never starts \
         situation-less — the empty-battle-race fix)",
    );

    gate::seed_full_load_gate(&mut app);
    let released = advance_until(&mut app, load_released, TRANSITION_BUDGET);
    assert!(
        released,
        "once a LoadedSituation is inserted, Load must release to Intro (or beyond) within \
         {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

fn bad_situation_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bad_situation_root")
}

#[test]
fn real_asset_failed_situation_falls_back_and_does_not_strand() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(bad_situation_root())
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<LoadedSituation>(&mut app, LOAD_SAFETY_NET);

    if let Some(loaded) = app.world().get_resource::<LoadedSituation>() {
        assert!(
            loaded.gangers.is_empty(),
            "the failure path must insert exactly the empty default situation (zero gangers)",
        );
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "even on a failed situation, Load must release to Intro (or beyond) with the \
         empty-default situation; last AppState was {:?}",
        app_state(&app),
    );
}
