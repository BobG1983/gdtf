//! GTW-216 (GTW-48 S2): the SHARED world-camera lifecycle.
//!
//! A single [`WorldCamera`]-marked `Camera2d` is spawned on entry to
//! `GameState::BattleScape` (by the presenter's `spawn_world_camera`, registered by the
//! app on `OnEnter(GameState::BattleScape)`) and despawned on its exit. It is configured
//! to render beneath the persistent GTW-120 UI camera (`Camera.order == -1`) on its own
//! non-zero render layer (so its `RenderLayers` does not intersect the UI camera's
//! default layer 0).
//!
//! These are headless `MinimalPlugins` ([`GdtfTestAppBuilder`]) tests: `Camera2d`,
//! `Camera`, and `RenderLayers` are plain components, queryable without a renderer —
//! exactly as `ui_camera_persists.rs` queries the UI camera. They are
//! *pin-discriminating*: each assertion re-encodes one acceptance criterion so a
//! regression turns the test red.
//!
//! Setup mirrors `battle_running_driver.rs`: the walk app injects the persistent `Load`
//! resources (`GdtfTheme` + `CombatTuning`) so the deep walk can traverse `Load` under
//! `MinimalPlugins` (no `AssetServer`); the walk then drives the state machine into and
//! out of `GameState::BattleScape`. `app.world_mut()` / `app.world()` in the TEST BODY is
//! the accepted headless idiom (`bevy-traps.md` #7 carve-out).
//!
//! The production `UiCamera` marker is module-private (`ui_camera.rs`), so AC3 asserts the
//! world camera's `RenderLayers` does not intersect `RenderLayers::layer(0)` DIRECTLY
//! rather than querying the UI camera entity.

use bevy::{
    camera::{Camera, visibility::RenderLayers},
    ecs::{entity::Entity, prelude::With},
    state::state::State,
};
use gdtf_app::test_support::{
    BattleRunningComplete, BattleScapeState, GameState, LoadedSituation, RunningState,
};
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{
    injuries::InjuryRegistry, level::ThemeCatalogRegistry, situation::Situation,
    terrain::piece::TerrainRegistry, tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into/out of the battlescape (each leaf
/// scene spends a couple of `FixedUpdate` ticks plus its transition propagation), but
/// bounded so a machine that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// Collects every entity that currently carries the [`WorldCamera`] marker.
fn world_cameras(app: &mut bevy::app::App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<WorldCamera>>()
        .iter(app.world())
        .collect()
}

/// Reads the current [`GameState`] if it is active.
fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless walk app, injecting the persistent `Load` resources the machine
/// needs to traverse `Load` (no `AssetServer` under `MinimalPlugins`). The
/// `LoadedSituation` seeded here is the empty default — the camera lifecycle reads no
/// sim state, so an empty battle suffices. Mirrors `battle_running_driver.rs::walk_app`.
fn walk_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-257: the Load->Intro gate also requires a WeaponRegistry (empty-default
    // situation here, so an empty registry clears the gate).
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-269: the Load->Intro gate also requires an ArmorRegistry; empty clears it (the
    // registry is dormant this slice — the setup does not read it yet).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    // GTW-394: the Load gate also requires a TerrainRegistry; empty clears it.
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut()
        .insert_resource(ThemeCatalogRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load->Intro gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-418: the Load gate also requires a PrefabRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    // GTW-261: the Load->Intro gate now also requires a LoadedSituation (the
    // empty-battle-race fix); seed the empty default beside the other three.
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
    app
}

/// Drives the app from the default start until [`GameState::BattleScape`] is active
/// (it rests in `BattleScapeState::Generation`). Stands in for the player at the menu
/// (which no longer auto-advances, GTW-121) by queuing `Menu → Options` once `Menu` rests.
fn drive_into_battlescape(app: &mut bevy::app::App) -> bool {
    let reached_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if !reached_menu {
        return false;
    }
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(
        app,
        |app| game_state(app) == Some(GameState::BattleScape),
        BUDGET,
    )
}

/// AC1 — exactly one `WorldCamera`-marked `Camera2d` exists while
/// `GameState::BattleScape` is active, and it is despawned on exit.
///
/// Drives into `BattleScape`, asserts exactly one `WorldCamera`, then drives out of
/// `GameState::BattleScape` and asserts the query is empty.
#[test]
fn world_camera_spawns_in_battlescape_and_despawns_on_exit() {
    let mut app = walk_app();
    assert!(
        drive_into_battlescape(&mut app),
        "the walk should reach GameState::BattleScape within {BUDGET} updates; last observed \
         GameState was {:?}",
        game_state(&app),
    );

    assert_eq!(
        world_cameras(&mut app).len(),
        1,
        "exactly one WorldCamera-marked Camera2d must exist while GameState::BattleScape is active",
    );

    // Drive out of GameState::BattleScape: the battlescape now PERSISTS in BattleRunning
    // (GTW-236, the placeholder budget auto-exit is gone), so insert the explicit
    // `BattleRunningComplete` end-signal marker (standing in for the not-yet-wired
    // victory/flee). Once the machine reaches BattleRunning the marker trips `move_on`,
    // and the chain advances out of the scape, firing OnExit(GameState::BattleScape) and
    // the despawn.
    app.world_mut().insert_resource(BattleRunningComplete);
    let left_battlescape = advance_until(
        &mut app,
        |app| game_state(app).is_none_or(|state| state != GameState::BattleScape),
        BUDGET,
    );
    assert!(
        left_battlescape,
        "the walk must leave GameState::BattleScape within {BUDGET} updates so the despawn fires; \
         last observed GameState was {:?}",
        game_state(&app),
    );

    assert_eq!(
        world_cameras(&mut app).len(),
        0,
        "the WorldCamera must be despawned on OnExit(GameState::BattleScape)",
    );
}

/// AC2 — the `WorldCamera` spans the `BattleScapeState` sub-states: it is NOT despawned
/// on any `BattleScapeState` `OnExit`.
///
/// Captures the single `WorldCamera` entity while resting in `Generation`, advances
/// through the `BattleScapeState` walk toward `BattleRunning`, and asserts the SAME entity
/// is still alive and the count is still exactly one. The
/// `ui_camera_persists.rs::ui_camera_survives_running_substate_exit` pattern retargeted to
/// the `BattleScapeState` span.
#[test]
fn world_camera_survives_battlescape_substate_transitions() -> Result<(), &'static str> {
    let mut app = walk_app();
    assert!(
        drive_into_battlescape(&mut app),
        "the walk should reach GameState::BattleScape within {BUDGET} updates",
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::Generation),
        "precondition: entering GameState::BattleScape rests in BattleScapeState::Generation",
    );

    let cameras_before = world_cameras(&mut app);
    assert_eq!(
        cameras_before.len(),
        1,
        "precondition: exactly one WorldCamera after entering GameState::BattleScape",
    );
    let camera = cameras_before
        .first()
        .copied()
        .ok_or("precondition: a WorldCamera entity must exist in GameState::BattleScape")?;

    // Advance through the BattleScapeState walk toward BattleRunning, firing the
    // intermediate sub-state OnExits (Generation, AnimateIn).
    let reached_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_running,
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The SAME camera must still be alive across the sub-state walk: count unchanged AND
    // the captured entity still present (not despawned by a sub-state OnExit).
    let cameras_after = world_cameras(&mut app);
    assert_eq!(
        cameras_after.len(),
        1,
        "the WorldCamera count must be unchanged across BattleScapeState sub-state transitions; \
         it must not be despawned on a BattleScapeState OnExit",
    );
    assert!(
        app.world().get_entity(camera).is_ok(),
        "the exact WorldCamera entity spawned on entry to GameState::BattleScape must still exist \
         after the BattleScapeState walk toward BattleRunning — proving it spans GameState::\
         BattleScape, not a sub-state",
    );

    Ok(())
}

/// AC3 — the `WorldCamera`'s `Camera.order == -1` and its `RenderLayers` does NOT
/// intersect layer 0 (the default layer the GTW-120 UI camera uses).
///
/// Queries `(&Camera, &RenderLayers)` `With<WorldCamera>` in `BattleScape` and asserts
/// `camera.order == -1` and `!render_layers.intersects(&RenderLayers::layer(0))`. Asserts
/// against the default layer-0 set DIRECTLY — the production `UiCamera` marker is
/// module-private and unqueryable.
#[test]
fn world_camera_renders_below_ui_and_off_layer_zero() {
    let mut app = walk_app();
    assert!(
        drive_into_battlescape(&mut app),
        "the walk should reach GameState::BattleScape within {BUDGET} updates",
    );

    let world = app.world_mut();
    let mut query = world.query_filtered::<(&Camera, &RenderLayers), With<WorldCamera>>();
    let configs: Vec<(isize, bool)> = query
        .iter(world)
        .map(|(camera, layers)| (camera.order, layers.intersects(&RenderLayers::layer(0))))
        .collect();

    assert_eq!(
        configs.len(),
        1,
        "exactly one WorldCamera with a Camera + RenderLayers must exist in GameState::BattleScape",
    );
    let Some(&(order, intersects_layer_zero)) = configs.first() else {
        return;
    };
    assert_eq!(
        order, -1,
        "the WorldCamera must render at Camera.order == -1 (beneath the UI camera at the default 0)",
    );
    assert!(
        !intersects_layer_zero,
        "the WorldCamera's RenderLayers must NOT intersect layer 0 (the UI camera's default layer)",
    );
}
