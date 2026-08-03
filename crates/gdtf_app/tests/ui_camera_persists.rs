use bevy::{
    camera::Camera2d,
    ecs::entity::Entity,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{AppState, RunningState};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};

const TRANSITION_BUDGET: u32 = 16;

fn camera2d_entities(app: &mut bevy::app::App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<Camera2d>>()
        .iter(app.world())
        .collect()
}

fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

#[test]
fn entering_running_spawns_exactly_one_camera() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();

    app.update();
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

#[test]
fn ui_camera_survives_running_substate_exit() -> Result<(), &'static str> {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();

    app.update();
    app.update();

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
