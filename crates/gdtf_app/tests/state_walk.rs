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
//! Since GTW-236, the battlescape PERSISTS in `BattleScapeState::BattleRunning`:
//! the placeholder 3-tick turn-budget auto-exit is gone, so the default deep walk
//! now RESTS at `BattleRunning` and only leaves once the explicit
//! `BattleRunningComplete` end-signal marker is inserted (the victory census / flee
//! button — not yet wired — are what insert it in the running game). These tests
//! stand in for that by inserting the marker through the `test_support` surface to
//! drive the chain past `BattleRunning`.
//!
//! The verified sequence (read off the per-scene `move_on` systems, with the
//! menu step now player-driven and the battlescape now persistence-gated):
//! `Init → Load → Intro → Running` at the top level; under `Running`,
//! `Menu →(player)→ Options → Game`; under `Game`, `Setup → HiveScape → BattleScape`;
//! under `BattleScape`,
//! `Generation → AnimateIn → BattleRunning →(rests; explicit BattleRunningComplete)→
//! AnimateOut → AfterMath`; under
//! `AfterMath`, `AnimateIn → DisplayAftermath → AnimateOut`, whose terminal pops
//! all the way out to `RunningState::Quit`, which advances `AppState` to
//! `Teardown`.

use bevy::{
    app::{App, AppExit},
    state::state::{NextState, State},
};
use gdtf_app::test_support::{
    AfterMathState, AppState, BattleRunningComplete, BattleScapeState, GameState, LoadedSituation,
    RunningState,
};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough for the whole deep walk (each leaf scene spends a
/// couple of `FixedUpdate` ticks plus its state-transition propagation), but
/// still bounded so a machine that never terminates fails instead of hanging.
const WALK_BUDGET: u32 = 64;

/// Builds the default-start headless walk app and seeds the four resources the
/// `Load` scene now requires (GTW-143 / GTW-206 / GTW-257 / GTW-261).
///
/// `Load` no longer advances on a frame-1 shortcut: it leaves only once a
/// [`GdtfTheme`](gdtf_ui::theme::GdtfTheme), a [`CombatTuning`], a
/// [`WeaponRegistry`], AND a [`LoadedSituation`] are all present, which the running
/// app resolves from the loose theme/tuning/weapons/situation RON via the
/// `AssetServer` (GTW-206 added the tuning, GTW-257 the registry, GTW-261 the
/// situation as required gate resources). The `MinimalPlugins` walk app has **no**
/// `AssetServer`, so this pre-inserts all four (standing in for the resolved loads)
/// so the walk can traverse `Load` and exercise the deep transition graph this file
/// is about. All four are the deliberate state-scoped-resource exceptions that
/// persist, so seeding them before the walk is faithful to how the real app carries
/// them forward.
fn walk_app_with_theme() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: the Load→Intro gate also requires a GangerStatTuning (the sim derives
    // ganger stats from it); the default clears the gate.
    app.world_mut().insert_resource(GangerStatTuning::default());
    // GTW-257: the Load→Intro gate also requires a WeaponRegistry (the deep walk uses
    // the empty-default situation, so an empty registry clears the gate).
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::AttachmentRegistry::default());
    // GTW-269: the Load→Intro gate also requires an ArmorRegistry; empty clears it (the
    // registry is dormant this slice — the setup does not read it yet).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it (a
    // real battle would resolve placed gangers against the loaded gangs folder).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the NEW gate-blocking PrefabRegistry2; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry2::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    // GTW-261: the Load→Intro gate now also requires a LoadedSituation (the
    // empty-battle-race fix). The headless walk has no AssetServer to resolve one, so
    // seed the empty default beside the other three — symmetric with theme/tuning/weapons.
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
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

/// Reads the current [`BattleScapeState`] if [`GameState::BattleScape`] is active.
fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
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

/// Drives the default-start walk all the way down to [`AppState::Teardown`] and returns
/// the app resting there, asserting each leg along the way (so a regression in the
/// transition graph fails in this shared driver, not just in one caller).
///
/// Standing in for the (not-yet-wired) player and victory/flee end conditions: it drives
/// past the menu (GTW-121), down to `BattleRunning` (GTW-236), verifies the battle
/// PERSISTS with no end-signal marker, then inserts the explicit `BattleRunningComplete`
/// marker which lets the deep terminal pop out to `Teardown`.
fn drive_to_teardown() -> App {
    let mut app = walk_app_with_theme();

    // The menu no longer auto-advances (GTW-121); stand in for the player to keep
    // the walk moving past it.
    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {WALK_BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    // The default walk now RESTS at BattleRunning (persistence, GTW-236).
    let reached_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        WALK_BUDGET,
    );
    assert!(
        reached_battle_running,
        "the walk should descend to BattleScapeState::BattleRunning within {WALK_BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // With no end-signal marker, the machine must NOT auto-advance off BattleRunning:
    // the 3-tick auto-exit is gone, so the battle persists across the whole budget.
    for _ in 0..WALK_BUDGET {
        app.update();
        assert_eq!(
            battlescape_state(&app),
            Some(BattleScapeState::BattleRunning),
            "BattleRunning must PERSIST with no BattleRunningComplete inserted (GTW-236); the \
             placeholder turn-budget auto-exit must not advance it",
        );
        assert_ne!(
            app_state(&app),
            AppState::Teardown,
            "the walk must NOT reach Teardown on its own — it rests at BattleRunning until an \
             explicit end signal",
        );
    }

    // Insert the explicit end-signal marker (standing in for victory/flee), then the
    // marker-gated `move_on` advances BattleRunning → AnimateOut → … and the deep
    // terminal ultimately pops out to Teardown.
    app.world_mut().insert_resource(BattleRunningComplete);
    let reached = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Teardown,
        WALK_BUDGET,
    );
    assert!(
        reached,
        "an explicit BattleRunningComplete insert should advance the walk to AppState::Teardown \
         within {WALK_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );

    app
}

/// (a) From the default start, the deep walk RESTS at
/// [`BattleScapeState::BattleRunning`] (it does NOT reach [`AppState::Teardown`] on
/// the walk alone, GTW-236) and reaches `Teardown` ONLY after an explicit
/// [`BattleRunningComplete`] insert stands in for the not-yet-wired victory/flee end
/// condition.
///
/// Pin: this fails if any top-level transition target regresses (so the walk stalls
/// before `BattleRunning`); it ALSO fails if the placeholder auto-exit ever returns
/// (the walk would reach `Teardown` on its own, before the explicit insert, and the
/// rest-at-`BattleRunning` assertion would catch the early advance). After the
/// insert, a regression in the deep terminal popping back out to `Teardown` keeps
/// the predicate unmet within budget.
#[test]
fn full_walk_reaches_teardown() {
    // The shared driver asserts the whole walk down to Teardown; reaching it without a
    // panic IS the assertion for this test.
    let app = drive_to_teardown();
    assert_eq!(
        app_state(&app),
        AppState::Teardown,
        "the driven walk should rest in AppState::Teardown",
    );
}

/// (a′) GTW-311: once the walk reaches [`AppState::Teardown`], the terminal `move_on`
/// system emits [`AppExit::Success`] within a bounded number of updates.
///
/// Pin: this is the regression that let the macOS shutdown hang (Bevy issue #23313)
/// ship — the prior `state_walk` reached `Teardown` but never asserted the exit fired,
/// so a `move_on` that silently failed to quit was invisible. This proves the HEADLESS
/// exit path (`exit.write(AppExit::Success)`) still fires: `App::should_exit()` reads
/// the `Messages<AppExit>` buffer, so checking it after each update catches the message
/// regardless of the message double-buffer's clear timing. The macOS window-despawn path
/// (winit consuming the close to terminate the loop) is NOT headless-testable — there is
/// no window under `MinimalPlugins` — and is verified by the in-engine playtest
/// (verification.md rule 3).
#[test]
fn teardown_emits_app_exit() {
    let mut app = drive_to_teardown();

    // After Teardown is entered, `teardown_complete` inserts the marker (1-tick deferred
    // Commands handoff) and then the marker-gated `move_on` writes `AppExit::Success`.
    // Poll `should_exit` each update so the assertion does not depend on the message
    // buffer's clear timing.
    let mut observed_exit = None;
    for _ in 0..WALK_BUDGET {
        app.update();
        if let Some(exit) = app.should_exit() {
            observed_exit = Some(exit);
            break;
        }
    }

    assert_eq!(
        observed_exit,
        Some(AppExit::Success),
        "Teardown's move_on must emit AppExit::Success within {WALK_BUDGET} updates of reaching \
         Teardown (GTW-311); observed {observed_exit:?}",
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

    // The battlescape now persists in BattleRunning (GTW-236) — drive down to it,
    // then insert the explicit end-signal marker (standing in for victory/flee) so
    // the marker-gated `move_on` advances the chain past BattleRunning.
    let reached_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        WALK_BUDGET,
    );
    assert!(
        reached_battle_running,
        "the walk should descend to BattleScapeState::BattleRunning within {WALK_BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app.world_mut().insert_resource(BattleRunningComplete);

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
