//! The GTW-48 capstone: headless proof that the WHOLE presenter + input + action-bar
//! stack composes to `BattleScapeState::BattleRunning`.
//!
//! GTW-749 moved the drive off the retired `GDTF_AUTOBATTLE` affordance onto the QA wire's
//! `StartBattle` request; GTW-943 deleted that request with the rest of the pre-command
//! vocabulary, so the drive is now the menu's own `StartBattleRequested` — the SAME message
//! the Battlescape button writes and `apply_start_battle` applies, one step earlier on the
//! path the wire used to join. The composition proof is unchanged: the real machine descends
//! `Menu → Game → … → BattleRunning`. `#![cfg(...)]` below keeps this file on the `net_qa`
//! feature, where its `NetQaPlugin` wiring lives; CI names the feature (GTW-883).
//!
//! Headless `GdtfTestAppBuilder` walk (the `action_bar.rs` / `battle_running_driver.rs`
//! precedent): `MinimalPlugins` + the real `ScenesPlugin` state machine, the persistent
//! `Load` resources injected because a headless app has no `AssetServer` to resolve them.
//! Every `app.world()` / `app.world_mut()` call is in a TEST BODY (the accepted headless
//! idiom, `bevy-traps.md` #7 carve-out (a)); no function here takes `&mut World` / `&World`.
#![cfg(all(debug_assertions, feature = "net_qa"))]

use std::sync::mpsc;

use bevy::{prelude::*, state::state::State};
use gdtf_app::test_support::{
    AimToggleButton, BattleScapeState, EndTurnButton, LevelDownButton, LevelUpButton,
    LoadedSituation, NetQaPlugin, RunningState, StanceKneelingButton, StanceProneButton,
    StanceStandingButton, StartBattleRequested,
};
use gdtf_battle_input::InspectTarget;
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{
    injuries::InjuryRegistry, situation::Situation, tuning::CombatTuning, weapon::WeaponRegistry,
};
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
/// a `CombatTuning`, a `WeaponRegistry`, and a `LoadedSituation`. This mirrors what the
/// real `Load` scene resolves from assets — an EMPTY situation (zero gangers), exactly
/// what the retired auto-battle affordance seeded, so the action-bar button count below
/// stays pinned to the no-selection shape.
fn seed_load(app: &mut App) {
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-257: the Load→Intro gate now also requires a WeaponRegistry; the empty default
    // LoadedSituation has zero gangers, so an empty registry clears the gate.
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    // GTW-269: the Load→Intro gate also requires an ArmorRegistry; empty clears it (the
    // registry is dormant this slice — the setup does not read it yet).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the NEW gate-blocking PrefabRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    // GTW-261: the Load→Intro gate now also requires a LoadedSituation (the empty-battle-
    // race fix). The headless walk has no AssetServer to resolve one, so seed the empty
    // default beside the other three — symmetric with theme/tuning/weapons.
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
}

/// ONE e2e test driving the full app — the real `ScenesPlugin` wires
/// `BattlePresenterPlugin::default()` (`TopDown`) + `GdtfBattleInputPlugin` +
/// `GameBattleScapeActionBarScenePlugin` — to `BattleScapeState::BattleRunning`, then
/// asserts the integrated wiring is LIVE. Each assert re-encodes one wiring fact
/// (pin-discriminating):
///
/// - the S2 `WorldCamera` spawned `OnEnter(GameState::BattleScape)`;
/// - the input crate's `InspectTarget` resource (`init_resource` ran);
/// - the action-bar's 5 existing-act buttons + 2 deferred buttons spawned
///   `OnEnter(BattleRunning)`;
/// - and the `PresenterSystems::Draw` wiring composes — the app `update()`s through
///   `BattleRunning` without panicking (under `MinimalPlugins` the atlas stack is
///   absent so the draw no-ops by design; the live sprite COUNT is the AC5 carve-out).
///
/// GTW-749, retargeted by GTW-943: the descent from the menu into the battle is driven by
/// the menu's OWN `StartBattleRequested` message + `apply_start_battle` — the same message
/// the local Battlescape button writes — replacing the retired `AutoBattlePlugin`. It used
/// to be driven by a `StartBattle` wire request; that request and its consumer are gone, so
/// the shared message is now the deepest point a QA command will reach. The app still adds
/// [`NetQaPlugin::with_channels`], so what this composes is a build with the QA channel
/// wired in.
#[test]
fn full_stack_composes_to_battle_running() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    seed_load(&mut app);
    let (_tx, rx) = mpsc::channel();
    app.add_plugins(NetQaPlugin::with_channels(rx));

    // Descend Init -> Load -> Intro -> Running -> Menu automatically.
    assert!(
        advance_until(
            &mut app,
            |app| running_state(app) == Some(RunningState::Menu),
            BUDGET,
        ),
        "the walk should reach RunningState::Menu within {BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    // Drive Menu -> Game -> ... -> BattleRunning through the menu's own start-battle
    // request — the same message the local Battlescape button writes.
    app.world_mut()
        .write_message(StartBattleRequested::new(None));
    app.update();

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

    // The input crate's InspectTarget resource is present (GdtfBattleInputPlugin's
    // init_resource ran inside the real ScenesPlugin).
    assert!(
        app.world().get_resource::<InspectTarget>().is_some(),
        "the input crate's InspectTarget resource must be present (GdtfBattleInputPlugin \
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
/// the 2 level buttons (the 4 control buttons), plus the 1 deferred end-turn button. (The
/// Reload deferred-button stub was REMOVED in GTW-275 — reload is now a LIVE button in the
/// weapon panel, not the action bar.) The Mode sub-panel's per-mode toggles are built on a
/// selection (none at spawn, when the e2e has no `SelectedShooter`), so they are not counted
/// here.
const EXPECTED_ACTION_BAR_BUTTONS: usize = 7;

/// Counts the stable action-bar buttons present by summing each marker. Re-encodes the
/// bar's composition so a regression that drops a button turns the e2e test red.
fn count_action_bar_buttons(app: &mut App) -> usize {
    count_marker::<StanceStandingButton>(app)
        + count_marker::<StanceKneelingButton>(app)
        + count_marker::<StanceProneButton>(app)
        + count_marker::<AimToggleButton>(app)
        + count_marker::<LevelUpButton>(app)
        + count_marker::<LevelDownButton>(app)
        + count_marker::<EndTurnButton>(app)
}

/// Counts entities carrying marker `M`.
fn count_marker<M: Component>(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<M>>();
    q.iter(world).count()
}
