//! GTW-223 (the GTW-48 capstone): headless proof of (1) the DEV auto-enter-battle
//! affordance's gate + drive logic, and (2) the WHOLE presenter + input + action-bar
//! stack composing to `BattleScapeState::BattleRunning`.
//!
//! Both are headless `GdtfTestAppBuilder` walks (the `action_bar.rs` /
//! `battle_running_driver.rs` precedent): `MinimalPlugins` + the real `ScenesPlugin`
//! state machine, the persistent `Load` resources injected because a headless app has
//! no `AssetServer` to resolve them. Every `app.world()` / `app.world_mut()` call is
//! in a TEST BODY (the accepted headless idiom, `bevy-traps.md` #7 carve-out (a)); no
//! function here takes `&mut World` / `&World`.
//!
//! The live GUI launch (`GDTF_AUTOBATTLE=1 cargo run …` landing a human in a rendered,
//! interactive battle) is the in-engine carve-out (AC5) the orchestrator's post-gate QA
//! supplies — these headless tests prove the gate logic + the stack composition the
//! per-slice tests could not consolidate, which is everything a headless harness can
//! observe.

use bevy::{prelude::*, state::state::State};
use gdtf_app::test_support::{
    AimToggleButton, AutoBattleActive, AutoBattlePlugin, BattleScapeState, EndTurnButton,
    LevelDownButton, LevelUpButton, LoadedSituation, ReloadButton, RunningState,
    StanceKneelingButton, StanceProneButton, StanceStandingButton, auto_battle_enabled,
};
// (no `GameState` import — the e2e test reads BattleScapeState directly.)
use gdtf_battle_input::HoveredCell;
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{situation::Situation, tuning::CombatTuning, weapon::WeaponRegistry};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape (each leaf scene
/// spends a couple of `FixedUpdate` ticks plus transition propagation), bounded so a
/// machine that never reaches the predicate fails instead of hanging (the
/// `battle_running_driver.rs` / `action_bar.rs` budget).
const BUDGET: u32 = 96;

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Seeds the four persistent `Load` resources a headless walk lacks (no
/// `AssetServer`), so the machine can traverse `Load` to the menu: a `default_theme`,
/// a `CombatTuning`, a `WeaponRegistry`, and a `LoadedSituation`. This mirrors what
/// the real `Load` scene resolves from assets and what the affordance itself seeds on
/// `Startup`; injecting them in BOTH the gate-OFF and gate-ON apps keeps the
/// affordance plugin the ONLY difference between them.
fn seed_load(app: &mut App) {
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-257: the Load→Intro gate now also requires a WeaponRegistry; the empty default
    // LoadedSituation has zero gangers, so an empty registry clears the gate.
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-261: the Load→Intro gate now also requires a LoadedSituation (the empty-battle-
    // race fix). The headless walk has no AssetServer to resolve one, so seed the empty
    // default beside the other three — symmetric with theme/tuning/weapons.
    app.world_mut()
        .insert_resource(LoadedSituation(Situation::default()));
}

// ---------------------------------------------------------------------------------
// AC1 — the affordance is inert by default and active under its gate.
// ---------------------------------------------------------------------------------

/// AC1 (gate predicate wired): the env-var gate is a pure function — `from_env` defers
/// to [`auto_battle_enabled`], and in the test process (which sets no `GDTF_AUTOBATTLE`)
/// the gate is OFF, so a plain build is inert. The recognised-spelling logic is unit-
/// checked in the module's own `#[cfg(test)]`; here we pin the WIRED relation: the
/// public gate predicate and the constructor agree.
#[test]
fn gate_predicate_is_wired_and_off_by_default() {
    assert_eq!(
        AutoBattlePlugin::from_env().enabled(),
        auto_battle_enabled(),
        "the affordance's construction-time gate must defer to the env-var predicate",
    );
    // The test process sets no GDTF_AUTOBATTLE, so the gate is OFF — the inert default.
    assert!(
        !auto_battle_enabled(),
        "with no GDTF_AUTOBATTLE set, the affordance must be inert by default",
    );
    // A forced-on plugin reports enabled; a forced-off one reports inert — the two
    // branches the drive tests below exercise deterministically.
    assert!(AutoBattlePlugin::with_enabled(true).enabled());
    assert!(!AutoBattlePlugin::with_enabled(false).enabled());
}

/// AC1 (gate-OFF drive): a `default_start` walk with NO affordance (and an inert /
/// disabled affordance) reaches `RunningState::Menu` and RESTS there — it never auto-
/// enters a battle within the budget. This re-encodes "normal launch reaches the menu
/// and STOPS".
#[test]
fn gate_off_rests_at_menu_never_reaching_battle() {
    let mut app = GdtfTestAppBuilder::new().default_start().build();
    // A disabled affordance must register nothing — adding it is indistinguishable from
    // a build without it (the inert-by-default contract).
    app.add_plugins(AutoBattlePlugin::with_enabled(false));
    seed_load(&mut app);

    // The machine descends Init -> Load -> Intro -> Running -> Menu automatically.
    assert!(
        advance_until(
            &mut app,
            |app| running_state(app) == Some(RunningState::Menu),
            BUDGET,
        ),
        "the inert walk should reach RunningState::Menu within {BUDGET} updates; last \
         observed RunningState was {:?}",
        running_state(&app),
    );

    // A disabled affordance inserts no AutoBattleActive witness.
    assert!(
        app.world().get_resource::<AutoBattleActive>().is_none(),
        "a disabled affordance must not insert the AutoBattleActive witness",
    );

    // And it STAYS at the menu: it never auto-advances into a battle.
    let reached_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        !reached_battle,
        "with the affordance inert, the menu must not auto-enter a battle; observed \
         BattleScapeState {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        running_state(&app),
        Some(RunningState::Menu),
        "the inert walk must REST at the menu",
    );
}

/// AC1 (gate-ON drive): the SAME walk with the affordance ACTIVE drives the real state
/// machine past the (no-auto-advance) menu and down to `BattleScapeState::BattleRunning`
/// — the `battle_running_driver.rs` drive, run by the app's own systems rather than the
/// test. It also proves the drive is self-disarming: once it nudges off the menu it
/// removes its `AutoBattleActive` witness (it fires exactly once).
#[test]
fn gate_on_drives_into_battle_running() {
    let mut app = GdtfTestAppBuilder::new().default_start().build();
    app.add_plugins(AutoBattlePlugin::with_enabled(true));
    // The affordance seeds these on Startup too; seeding here matches the gate-OFF app so
    // the ONLY difference between the two is the affordance's activation.
    seed_load(&mut app);

    assert!(
        advance_until(
            &mut app,
            |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
            BUDGET,
        ),
        "the ACTIVE affordance must drive the machine to BattleScapeState::BattleRunning \
         within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The drive disarmed itself the moment it left the menu (fires exactly once).
    assert!(
        app.world().get_resource::<AutoBattleActive>().is_none(),
        "the drive must remove the AutoBattleActive witness after nudging off the menu",
    );
    // It is no longer parked at the menu — it advanced through Game into the battle.
    assert_ne!(
        running_state(&app),
        Some(RunningState::Menu),
        "the active affordance must have advanced past the menu",
    );
}

/// AC1 (the seed is wired): the active affordance's `Startup` seed makes the walk
/// reach the battle even WITHOUT a test-body `seed_load` — it provides its own `Load`
/// fallbacks (`default_theme()` + `CombatTuning` + a default `LoadedSituation`), so the
/// drive is self-contained, exactly as the GUI launch needs.
#[test]
fn active_affordance_seeds_its_own_load_fallbacks() {
    let mut app = GdtfTestAppBuilder::new().default_start().build();
    app.add_plugins(AutoBattlePlugin::with_enabled(true));
    // No seed_load here — the affordance's Startup seed must supply the Load resources.

    assert!(
        advance_until(
            &mut app,
            |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
            BUDGET,
        ),
        "the active affordance must seed its own Load fallbacks and still reach \
         BattleRunning within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    // CombatTuning is one of the resources only the affordance's seed could have provided
    // in this no-test-seed app — its presence proves the Startup seed ran.
    assert!(
        app.world().get_resource::<CombatTuning>().is_some(),
        "the affordance's Startup seed must have inserted CombatTuning",
    );
}

// ---------------------------------------------------------------------------------
// AC2 — the WHOLE presenter + input + action-bar stack composes to BattleRunning.
// ---------------------------------------------------------------------------------

/// AC2: ONE e2e test driving the full app — the real `ScenesPlugin` wires
/// `BattlePresenterPlugin::default()` (`TopDown`) + `GdtfBattleInputPlugin` +
/// `GameBattleScapeActionBarScenePlugin` — to `BattleScapeState::BattleRunning`, then
/// asserts the integrated wiring is LIVE. Each assert re-encodes one wiring fact
/// (pin-discriminating):
///
/// - the S2 `WorldCamera` spawned `OnEnter(GameState::BattleScape)`;
/// - the input crate's `HoveredCell` resource (`init_resource` ran);
/// - the action-bar's 5 existing-act buttons + 2 deferred buttons spawned
///   `OnEnter(BattleRunning)`;
/// - and the `PresenterSystems::Draw` wiring composes — the app `update()`s through
///   `BattleRunning` without panicking (under `MinimalPlugins` the atlas stack is
///   absent so the draw no-ops by design; the live sprite COUNT is the AC5 carve-out).
#[test]
fn full_stack_composes_to_battle_running() {
    let mut app = GdtfTestAppBuilder::new().default_start().build();
    // The affordance drives the walk for us (the same drive the GUI uses). Seeding via
    // the affordance keeps this test exercising the affordance + the full stack together.
    app.add_plugins(AutoBattlePlugin::with_enabled(true));

    assert!(
        advance_until(
            &mut app,
            |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
            BUDGET,
        ),
        "the full stack must compose and reach BattleRunning within {BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The S2 world camera is present (spawned OnEnter(GameState::BattleScape)).
    let world_cameras = {
        let world = app.world_mut();
        let mut q = world.query::<&WorldCamera>();
        q.iter(world).count()
    };
    assert_eq!(
        world_cameras, 1,
        "exactly one WorldCamera must be present in BattleRunning (the S2 camera \
         lifecycle ran)",
    );

    // The input crate's HoveredCell resource is present (GdtfBattleInputPlugin's
    // init_resource ran inside the real ScenesPlugin).
    assert!(
        app.world().get_resource::<HoveredCell>().is_some(),
        "the input crate's HoveredCell resource must be present (GdtfBattleInputPlugin \
         is wired into the real stack)",
    );

    // The action-bar buttons are present — the 3 stance toggles + aim + 2 level + 2
    // deferred buttons spawned OnEnter(BattleRunning) (the Mode toggles build on selection).
    assert_eq!(
        count_action_bar_buttons(&mut app),
        EXPECTED_ACTION_BAR_BUTTONS,
        "all {EXPECTED_ACTION_BAR_BUTTONS} action-bar buttons must be spawned in \
         BattleRunning",
    );

    // The draw wiring composes: another update through BattleRunning must not panic
    // (the PresenterSystems::Draw band ran; under MinimalPlugins it no-ops on the
    // absent atlas stack by design — the sprite COUNT is the AC5 in-engine carve-out).
    app.update();
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "the stack must keep running in BattleRunning across an extra update without \
         panicking",
    );
}

/// The number of STABLE action-bar buttons spawned in `BattleRunning` at
/// `OnEnter(BattleRunning)`: the 3 stance toggles (Stand / Kneel / Prone), the aim toggle,
/// and the 2 level buttons (the 4 control buttons), plus the 2 deferred buttons (reload,
/// end-turn). The Mode sub-panel's per-mode toggles are built on a selection (none at
/// spawn, when the e2e has no `SelectedShooter`), so they are not counted here.
const EXPECTED_ACTION_BAR_BUTTONS: usize = 8;

/// Counts the stable action-bar buttons present by summing each marker. Re-encodes the
/// bar's composition so a regression that drops a button turns the e2e test red.
fn count_action_bar_buttons(app: &mut App) -> usize {
    count_marker::<StanceStandingButton>(app)
        + count_marker::<StanceKneelingButton>(app)
        + count_marker::<StanceProneButton>(app)
        + count_marker::<AimToggleButton>(app)
        + count_marker::<LevelUpButton>(app)
        + count_marker::<LevelDownButton>(app)
        + count_marker::<ReloadButton>(app)
        + count_marker::<EndTurnButton>(app)
}

/// Counts entities carrying marker `M`.
fn count_marker<M: Component>(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<M>>();
    q.iter(world).count()
}
