//! Focus nav: set initial focus, directional walk, FocusCancelled registration.
use bevy::{
    ecs::{
        message::{MessageReader, Messages},
        system::RunSystemOnce,
    },
    input_focus::{InputFocus, directional_navigation::DirectionalNavigationMap},
    math::CompassOctant,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::focus_nav::{FocusCancelled, NavDirection, NavigateRequest, set_initial_focus};

#[test]
fn set_initial_focus_sets_the_start() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(gdtf_app::test_support::AppState::Running)
        .build();
    app.update();

    let world = app.world_mut();
    let target = world.spawn_empty().id();

    let mut commands = world.commands();
    set_initial_focus(&mut commands, target);
    world.flush();

    assert_eq!(
        world.resource::<InputFocus>().get(),
        Some(target),
        "set_initial_focus must point InputFocus at the given entity",
    );
}

#[test]
fn navigate_down_advances_focus() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(gdtf_app::test_support::AppState::Running)
        .build();
    app.update();

    let world = app.world_mut();
    let top = world.spawn_empty().id();
    let bottom = world.spawn_empty().id();

    world
        .resource_mut::<DirectionalNavigationMap>()
        .add_symmetrical_edge(top, bottom, CompassOctant::South);

    set_initial_focus(&mut world.commands(), top);
    world.flush();
    assert_eq!(
        world.resource::<InputFocus>().get(),
        Some(top),
        "focus should start on the top node before navigating",
    );

    world.write_message(NavigateRequest::new(NavDirection::DOWN));
    app.update();

    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(bottom),
        "navigate-Down must advance InputFocus to the bottom (South) neighbor",
    );
}

#[test]
fn navigate_east_then_west_walks_focus_horizontally() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(gdtf_app::test_support::AppState::Running)
        .build();
    app.update();

    let world = app.world_mut();
    let left = world.spawn_empty().id();
    let right = world.spawn_empty().id();

    world
        .resource_mut::<DirectionalNavigationMap>()
        .add_symmetrical_edge(left, right, CompassOctant::East);

    set_initial_focus(&mut world.commands(), left);
    world.flush();
    assert_eq!(
        world.resource::<InputFocus>().get(),
        Some(left),
        "focus should start on the left node before navigating",
    );

    world.write_message(NavigateRequest::new(NavDirection::EAST));
    app.update();
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(right),
        "navigate-East must advance InputFocus to the right (East) neighbor",
    );

    app.world_mut()
        .write_message(NavigateRequest::new(NavDirection::WEST));
    app.update();
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(left),
        "navigate-West must walk InputFocus back to the left (West) neighbor",
    );
}

fn count_focus_cancelled(mut reader: MessageReader<FocusCancelled>) -> usize {
    reader.read().count()
}

#[test]
fn focus_cancelled_message_is_registered_and_round_trips() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(gdtf_app::test_support::AppState::Running)
        .build();
    app.update();

    assert!(
        app.world()
            .get_resource::<Messages<FocusCancelled>>()
            .is_some(),
        "FocusNavPlugin must register the FocusCancelled message buffer",
    );

    app.world_mut().write_message(FocusCancelled);
    let seen = app
        .world_mut()
        .run_system_once(count_focus_cancelled)
        .unwrap_or(0);
    assert_eq!(
        seen, 1,
        "a FocusCancelled written to the framework buffer must be readable back",
    );
}
