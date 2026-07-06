//! Which scene each state hosts: `Running` hosts the menu, `Game` hosts setup.

use bevy::state::state::State;
use gdtf_app::test_support::{AppState, GameState, RunningState, app_state};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};

use super::harness::*;

/// Reads the current [`GameState`] if [`RunningState::Game`] is active.
fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

/// (b) Entering [`AppState::Running`] hosts its default child
/// [`RunningState::Menu`] — the parent does not auto-advance past it.
///
/// Pin: this is the core GTW-112 fix. Before the fix, the `Running` parent
/// scene auto-advanced and the default child `Menu` was never the resting
/// state after the first update. If a parent scene regains an auto-advancing
/// `move_on`, `RunningState` will already have moved off `Menu` here and the
/// assertion fails.
#[test]
fn running_hosts_menu() {
    // GTW-322 — entering `Running` enters `Menu`, whose `spawn_menu` now authors its tree
    // via `bsn!` / `spawn_scene`; the deferred apply needs the scene resources, so build
    // with scene support (matching this file's other walk helpers).
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();

    app.update();

    assert_eq!(
        app_state(&app),
        AppState::Running,
        "starting_in(Running) should rest in AppState::Running after one update",
    );
    assert_eq!(
        running_state(&app),
        Some(RunningState::Menu),
        "AppState::Running must host its default child RunningState::Menu, not auto-advance past it",
    );
}

/// (c) Driving into [`RunningState::Game`] activates its default child
/// [`GameState::Setup`].
///
/// Pin: `GameState` only exists while `RunningState::Game` is active (it is a
/// sub-state sourced on `Game`). This asserts the walk actually reaches `Game`
/// *and* that entering `Game` brings up `Setup` as the hosted default child. It
/// fails if the `Menu → Options → Game` chain regresses (so `Game` is never
/// reached within budget) or if `Game` auto-advances past `Setup`.
#[test]
fn game_hosts_setup() {
    // GTW-322 — see `running_hosts_menu`: the `Menu` `spawn_menu` scene needs the scene
    // resources, so build with scene support.
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();

    // The menu no longer auto-advances (GTW-121); stand in for the player.
    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    let reached = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Game),
        WALK_BUDGET,
    );
    assert!(
        reached,
        "the Menu →(player)→ Options → Game chain should reach RunningState::Game within \
         {WALK_BUDGET} updates; last observed RunningState was {:?}",
        running_state(&app),
    );

    assert_eq!(
        game_state(&app),
        Some(GameState::Setup),
        "entering RunningState::Game must activate its default child GameState::Setup",
    );
}
