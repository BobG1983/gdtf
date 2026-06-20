//! GTW-215 (GTW-48 S1): the AC3 headless integration test — proof that
//! `GameBattleScapeScenePlugin` adds `gdtf_battle_presenter::BattlePresenterPlugin`
//! and that the presenter's `build` actually RAN inside the real scene stack.
//!
//! It reuses the E10.7 capstone (`battle_bootstrap.rs`) drive pattern EXACTLY:
//! a [`GdtfTestAppBuilder`] (`MinimalPlugins` + the real `ScenesPlugin` state
//! machine) started at [`AppState::Running`], seeded with the persistent `Load`
//! resources a `MinimalPlugins` app cannot resolve from assets
//! ([`GdtfTheme`](gdtf_ui::theme::GdtfTheme) via [`default_theme`] +
//! [`CombatTuning`], GTW-143 / GTW-206), driven past the no-longer-auto-advancing
//! menu (GTW-121) by queuing `Menu -> Options`, then advanced under a bounded
//! budget until the world holds a `State<GameState>` == `BattleScape`.
//!
//! `BattlePresenterPlugin` is added by `GameBattleScapeScenePlugin`'s `build`, which
//! runs when the scene plugins register (not on state-enter), so its
//! [`TopDownRendererActive`](gdtf_battle_presenter::TopDownRendererActive) marker is
//! present once the app has descended to `BattleScape` — the test-observable proof
//! the build ran inside the real stack.
//!
//! NO function in this file takes `&mut World`/`&World`; every `app.world_mut()` /
//! `app.world()` call is in the TEST BODY (the established `gdtf_app` test idiom,
//! `bevy-traps.md` #7 carve-out a). This slice seeds NO `LoadedSituation`
//! (`request_battle_setup` falls back to `Situation::default()`), spawns no camera,
//! and stops at `BattleScape`.

use gdtf_app::test_support::{AppState, GameState, RunningState};
use gdtf_battle_presenter::TopDownRendererActive;
use gdtf_battle_sim::{tuning::CombatTuning, weapon::WeaponRegistry};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk down into the battlescape (each leaf
/// scene spends a couple of `FixedUpdate` ticks plus its transition propagation), but
/// bounded so a machine that never reaches the predicate fails instead of hanging —
/// the capstone's value.
const BUDGET: u32 = 96;

/// Reads the current [`GameState`] if it is active.
fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<bevy::state::state::State<GameState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<bevy::state::state::State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless app at [`AppState::Running`], seeding the persistent `Load`
/// resources the machine needs to traverse `Load` under `MinimalPlugins` (no
/// `AssetServer`): [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) via [`default_theme`] +
/// [`CombatTuning`] (the `battle_bootstrap` / `state_walk` precedent). No
/// `LoadedSituation` is seeded — the Generation setup falls back to the default
/// (empty) situation, which is sufficient to reach `BattleScape`.
fn presenter_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-257: the Load→Intro gate also requires a WeaponRegistry (no LoadedSituation
    // here, so the empty-default setup needs no weapons — an empty registry clears it).
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-269: the Load gate also requires an ArmorRegistry; empty clears it (the
    // registry is dormant this slice — the setup does not read it yet).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app
}

/// Stands in for the player at the menu (it no longer auto-advances, GTW-121):
/// advances until [`RunningState::Menu`] rests, then queues `Menu -> Options`.
fn drive_past_menu(app: &mut bevy::app::App) -> bool {
    let reached = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if reached {
        app.world_mut()
            .resource_mut::<bevy::state::state::NextState<RunningState>>()
            .set(RunningState::Options);
    }
    reached
}

/// Drives the app from [`AppState::Running`] down to the first update on which
/// [`GameState::BattleScape`] is active. Returns whether it was reached.
fn drive_to_battlescape(app: &mut bevy::app::App) -> bool {
    if !drive_past_menu(app) {
        return false;
    }
    advance_until(
        app,
        |app| game_state(app) == Some(GameState::BattleScape),
        BUDGET,
    )
}

/// AC1 — `GameBattleScapeScenePlugin` adds `BattlePresenterPlugin`, and its `build`
/// actually runs inside the real scene stack. Driving the real state machine into
/// [`GameState::BattleScape`] leaves the
/// [`TopDownRendererActive`](gdtf_battle_presenter::TopDownRendererActive) marker
/// present in the world — proof the default (`TopDown`) presenter's `build` ran
/// inside the real scene stack, not in an isolated test app.
#[test]
fn battlescape_scene_runs_the_presenter_plugin_build() {
    let mut app = presenter_app();
    assert!(
        drive_to_battlescape(&mut app),
        "the walk should descend to GameState::BattleScape within {BUDGET} updates; last observed \
         GameState was {:?}",
        game_state(&app),
    );
    assert_eq!(
        game_state(&app),
        Some(GameState::BattleScape),
        "the walk must rest inside GameState::BattleScape",
    );
    assert!(
        app.world()
            .get_resource::<TopDownRendererActive>()
            .is_some(),
        "BattlePresenterPlugin (default TopDown) build must have run inside the real scene stack — \
         the TopDownRendererActive marker is present once BattleScape is active",
    );
}
