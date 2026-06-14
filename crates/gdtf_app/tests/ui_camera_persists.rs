//! GTW-120 regression: the persistent UI [`Camera2d`] spawned on entry to
//! [`AppState::Running`] survives `RunningState` sub-state transitions.
//!
//! These are *pin-discriminating* tests. The camera is spawned by
//! `spawn_ui_camera` on `OnEnter(AppState::Running)` with **no** `DespawnOnExit`
//! (or any scene-scoped cleanup) marker, so it must outlive every `RunningState`
//! transition. If a regression attached a state-scoped despawn to the camera (or
//! moved its spawn under a sub-state), the survival assertion below would see the
//! camera count drop to zero / the captured entity vanish and turn red.
//!
//! Built on the `MinimalPlugins` [`GdtfTestAppBuilder`] — `Camera2d` is just a
//! component here, so spawning and counting it needs no renderer and asserts
//! **presence/survival only**, never layout geometry (which `MinimalPlugins`
//! does not compute).

use bevy::{
    camera::Camera2d,
    ecs::entity::Entity,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{AppState, RunningState};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};

/// Budget for the (single-step) `RunningState` transition off `Menu` plus its
/// state-transition propagation — bounded so a machine that never leaves `Menu`
/// fails instead of hanging.
const TRANSITION_BUDGET: u32 = 16;

/// Collects every entity that currently carries a [`Camera2d`].
///
/// Queries the world directly (the production marker on the camera is module-
/// private), so this counts *all* `Camera2d` entities — under `MinimalPlugins`
/// the only one is the production UI camera, so this is an exact census of it.
fn camera2d_entities(app: &mut bevy::app::App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<Camera2d>>()
        .iter(app.world())
        .collect()
}

/// Reads the current [`RunningState`] if [`AppState::Running`] is active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// On entry to [`AppState::Running`], exactly one [`Camera2d`] exists.
///
/// Pin: fails if `spawn_ui_camera` is removed from the
/// `OnEnter(AppState::Running)` wiring (count `0`) or if it spawns more than one
/// camera (count `> 1`).
#[test]
fn entering_running_spawns_exactly_one_camera() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();

    app.update();

    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Running,
        "starting_in(Running) should rest in AppState::Running after one update",
    );
    assert_eq!(
        camera2d_entities(&mut app).len(),
        1,
        "entering AppState::Running must spawn exactly one Camera2d (the persistent UI camera)",
    );
}

/// Driving `OnExit(RunningState::Menu)` leaves the **same** [`Camera2d`] alive —
/// the count is unchanged and the originally-spawned entity still exists.
///
/// Pin: this is the core GTW-120 guarantee. The camera is spawned tied to
/// `AppState::Running`, not to any `RunningState` sub-state, and carries no
/// scene-scoped despawn marker. If a regression scoped the camera to
/// `RunningState::Menu` (e.g. `DespawnOnExit(RunningState::Menu)`) or respawned
/// it per sub-state, then after the `Menu → Options` transition the captured
/// entity would be despawned (or replaced) and this assertion fails.
#[test]
fn ui_camera_survives_running_substate_exit() -> Result<(), &'static str> {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();

    app.update();

    // Capture the exact camera entity spawned on entry to Running.
    let cameras_before = camera2d_entities(&mut app);
    assert_eq!(
        cameras_before.len(),
        1,
        "precondition: exactly one Camera2d after entering AppState::Running",
    );
    let camera = cameras_before
        .first()
        .copied()
        .ok_or("precondition: a Camera2d entity must exist after entering AppState::Running")?;
    assert_eq!(
        running_state(&app),
        Some(RunningState::Menu),
        "precondition: AppState::Running hosts its default child RunningState::Menu",
    );

    // Explicitly drive the sub-state off Menu so `OnExit(RunningState::Menu)`
    // fires, then wait for the transition to actually apply.
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    let left_menu = advance_until(
        &mut app,
        |app| running_state(app).is_some_and(|state| state != RunningState::Menu),
        TRANSITION_BUDGET,
    );
    assert!(
        left_menu,
        "RunningState should leave Menu (firing OnExit(RunningState::Menu)) within \
         {TRANSITION_BUDGET} updates; last observed RunningState was {:?}",
        running_state(&app),
    );

    // The SAME camera must still be alive: count unchanged AND the captured
    // entity still present (not despawned/replaced by a sub-state transition).
    let cameras_after = camera2d_entities(&mut app);
    assert_eq!(
        cameras_after.len(),
        1,
        "the persistent UI camera count must be unchanged across OnExit(RunningState::Menu); \
         it must not be despawned by a scene-scoped cleanup",
    );
    assert!(
        app.world().get_entity(camera).is_ok(),
        "the exact Camera2d entity spawned on entry to Running must still exist after \
         OnExit(RunningState::Menu) — proving it carries no state-scoped despawn marker",
    );

    Ok(())
}
