//! GTW-112 regression: the full `AppState` state-walk drives from the default
//! start all the way to [`AppState::Teardown`] without a parent scene
//! auto-advancing past its hosted default child and without the machine
//! wrapping or looping.
//!
//! These are *pin-discriminating* tests: each assertion re-encodes a specific
//! GTW-112 fix so that a regression in a transition target, or a parent scene
//! that auto-advances again, turns the test red rather than letting the walk
//! silently take a wrong path.
//!
//! Since GTW-143, `Load` no longer advances on a frame-1 shortcut — it leaves
//! only once a `GdtfTheme` is present (resolved from assets in the real app).
//! The `MinimalPlugins` walk has no `AssetServer`, so [`walk_app_with_theme`]
//! seeds the theme before the walk; the deep transition graph below is otherwise
//! unchanged.
//!
//! Since GTW-121, `RunningState::Menu` no longer auto-advances either — the menu
//! now spawns a real screen and waits for player action (the button transitions
//! are GTW-122). The `MinimalPlugins` walk has no input, so the helper
//! [`drive_past_menu`] stands in for the player by queuing the `Menu → Options`
//! transition once the walk is resting on `Menu`, after which the rest of the
//! chain (`Options → Game → …`) still auto-advances through its scaffolds.
//!
//! The verified sequence (read off the per-scene `move_on` systems, with the
//! menu step now player-driven):
//! `Init → Load → Intro → Running` at the top level; under `Running`,
//! `Menu →(player)→ Options → Game`; under `Game`, `Setup → HiveScape → BattleScape`;
//! under `BattleScape`,
//! `Generation → AnimateIn → BattleRunning → AnimateOut → AfterMath`; under
//! `AfterMath`, `AnimateIn → DisplayAftermath → AnimateOut`, whose terminal pops
//! all the way out to `RunningState::Quit`, which advances `AppState` to
//! `Teardown`.

use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{AfterMathState, AppState, BattleScapeState, GameState, RunningState};
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough for the whole deep walk (each leaf scene spends a
/// couple of `FixedUpdate` ticks plus its state-transition propagation), but
/// still bounded so a machine that never terminates fails instead of hanging.
const WALK_BUDGET: u32 = 64;

/// Builds the default-start headless walk app and seeds the [`GdtfTheme`] and
/// [`CombatTuning`] the `Load` scene now requires (GTW-143 / GTW-206).
///
/// `Load` no longer advances on a frame-1 shortcut: it leaves only once BOTH a
/// [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) and a [`CombatTuning`] are present,
/// which the running app resolves from the loose theme/tuning RON via the
/// `AssetServer` (GTW-206 / E10.4 AC5 added the tuning as the second required
/// resource). The `MinimalPlugins` walk app has **no** `AssetServer`, so this
/// pre-inserts both (standing in for the resolved loads) so the walk can traverse
/// `Load` and exercise the deep transition graph this file is about. Both are the
/// deliberate state-scoped-resource exceptions that persist, so seeding them
/// before the walk is faithful to how the real app carries them forward.
fn walk_app_with_theme() -> App {
    let mut app = GdtfTestAppBuilder::new().default_start().build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app
}

/// Reads the current [`AppState`]. `AppState` is `Clone` but not `Copy`, so this
/// clones the resting value out of the `State<AppState>` resource.
fn app_state(app: &bevy::app::App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// Reads the current [`RunningState`] if [`AppState::Running`] is active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`GameState`] if [`RunningState::Game`] is active.
fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

/// Stands in for the player at the menu: advances the walk until
/// [`RunningState::Menu`] is reached, then queues the `Menu → Options`
/// transition (the menu no longer auto-advances since GTW-121).
///
/// Returns whether `Menu` was reached and the transition queued within budget.
/// After this returns `true`, one more `update()` (driven by the caller's
/// `advance_until`) applies the queued `NextState` and the rest of the chain
/// continues through its scaffolds.
fn drive_past_menu(app: &mut App) -> bool {
    let reached_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        WALK_BUDGET,
    );
    if reached_menu {
        app.world_mut()
            .resource_mut::<NextState<RunningState>>()
            .set(RunningState::Options);
    }
    reached_menu
}

/// (a) From the default start, the machine reaches [`AppState::Teardown`]
/// within the bounded budget.
///
/// Pin: this fails if any top-level transition target regresses (so the walk
/// stalls before `Teardown`) or if the deep terminal stops popping back out to
/// `Teardown`. A machine that wraps/loops instead of terminating also fails,
/// because it never satisfies the predicate within the budget.
#[test]
fn full_walk_reaches_teardown() {
    let mut app = walk_app_with_theme();

    // The menu no longer auto-advances (GTW-121); stand in for the player to keep
    // the walk moving past it.
    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    let reached = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Teardown,
        WALK_BUDGET,
    );

    assert!(
        reached,
        "full walk from the default start should reach AppState::Teardown within {WALK_BUDGET} \
         updates; last observed AppState was {:?}",
        app_state(&app),
    );
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
    let mut app = GdtfTestAppBuilder::new()
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
    let mut app = GdtfTestAppBuilder::new()
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

/// (d) The deep terminal pops out correctly: the `AfterMath` sub-machine
/// finishes into [`RunningState::Quit`], and only then does [`AppState`] reach
/// [`Teardown`] — the machine does not wrap back to an earlier state.
///
/// Pin: this asserts the exact deep-pop order. First it confirms the deepest
/// sub-state [`AfterMathState::AnimateOut`] is actually visited (proving the
/// walk descends the whole battlescape/aftermath chain rather than short-
/// circuiting). Then it confirms `RunningState` reaches `Quit` (the terminal
/// pop) and that `AppState` reaches `Teardown`. If the aftermath terminal
/// regressed to re-enter the game (a wrap/loop) instead of popping to `Quit`,
/// `RunningState::Quit` would never be observed and the assertion fails.
#[test]
fn deep_pop_to_quit() {
    let mut app = walk_app_with_theme();

    // The menu no longer auto-advances (GTW-121); stand in for the player.
    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    // Prove the walk descends all the way into the deepest aftermath phase.
    let reached_aftermath_out = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<BattleScapeState>>()
                .is_some_and(|state| *state.get() == BattleScapeState::AfterMath)
                && app
                    .world()
                    .get_resource::<State<AfterMathState>>()
                    .is_some_and(|state| *state.get() == AfterMathState::AnimateOut)
        },
        WALK_BUDGET,
    );
    assert!(
        reached_aftermath_out,
        "the walk should descend into BattleScapeState::AfterMath / AfterMathState::AnimateOut \
         within {WALK_BUDGET} updates",
    );

    // The aftermath terminal pops out to RunningState::Quit (not back into Game).
    let reached_quit = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Quit),
        WALK_BUDGET,
    );
    assert!(
        reached_quit,
        "the AfterMath terminal should pop all the way out to RunningState::Quit, not wrap back \
         into the game; last observed RunningState was {:?}",
        running_state(&app),
    );

    // And Quit advances the top-level machine to Teardown — the terminal end.
    let reached_teardown = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Teardown,
        WALK_BUDGET,
    );
    assert!(
        reached_teardown,
        "RunningState::Quit should advance AppState to Teardown; last observed AppState was {:?}",
        app_state(&app),
    );
}
