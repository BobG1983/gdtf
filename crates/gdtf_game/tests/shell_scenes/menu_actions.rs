//! Main menu actions: button press and focus activation request the mapped state.
use bevy::{
    ecs::entity::Entity,
    state::state::{NextState, State},
    ui::Interaction,
};
use gdtf_game::test_support::{
    AppState, BattlescapeButton, HiveScapeButton, OptionsButton, QuitButton, RunningState,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::{DisabledButton, focus_nav::FocusActivated, theme::default_theme};

fn menu_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.update();
    app
}

fn single_with<M: bevy::ecs::component::Component>(app: &mut bevy::app::App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

fn pending_state(app: &bevy::app::App) -> Option<RunningState> {
    match app.world().resource::<NextState<RunningState>>() {
        NextState::Pending(state) | NextState::PendingIfNeq(state) => Some(*state),
        NextState::Unchanged => None,
    }
}

fn set_interaction(app: &mut bevy::app::App, button: Entity, state: Interaction) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = state;
    }
}

#[test]
fn mouse_press_on_enabled_button_requests_mapped_transition() {
    for (label, marker_lookup, expected) in enabled_cases() {
        let mut app = menu_app();
        let button = marker_lookup(&mut app).unwrap_or(Entity::PLACEHOLDER);

        set_interaction(&mut app, button, Interaction::Pressed);
        app.update();

        assert_eq!(
            pending_state(&app),
            Some(expected),
            "mouse press on {label} must request RunningState::{expected:?}",
        );
    }
}

#[test]
fn focus_activation_on_enabled_button_requests_mapped_transition() {
    for (label, marker_lookup, expected) in enabled_cases() {
        let mut app = menu_app();
        let button = marker_lookup(&mut app).unwrap_or(Entity::PLACEHOLDER);

        app.world_mut().write_message(FocusActivated::new(button));
        app.update();

        assert_eq!(
            pending_state(&app),
            Some(expected),
            "focus activation on {label} must request RunningState::{expected:?}",
        );
    }
}

#[test]
fn disabled_hivescape_produces_no_transition_on_any_input() {
    let mut app = menu_app();
    let hivescape = single_with::<HiveScapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    assert!(
        app.world().get::<DisabledButton>(hivescape).is_some(),
        "precondition: HiveScape must carry DisabledButton",
    );

    set_interaction(&mut app, hivescape, Interaction::Pressed);
    app.world_mut()
        .write_message(FocusActivated::new(hivescape));
    app.update();

    assert_eq!(
        pending_state(&app),
        None,
        "disabled HiveScape must request no RunningState transition under any input",
    );
    assert_eq!(
        app.world().resource::<State<RunningState>>().get(),
        &RunningState::Menu,
        "and the menu must remain on RunningState::Menu",
    );
}

type MarkerLookup = fn(&mut bevy::app::App) -> Option<Entity>;

type EnabledCase = (&'static str, MarkerLookup, RunningState);

fn enabled_cases() -> [EnabledCase; 3] {
    [
        (
            "Battlescape",
            single_with::<BattlescapeButton>,
            RunningState::Game,
        ),
        (
            "Options",
            single_with::<OptionsButton>,
            RunningState::Options,
        ),
        ("Quit", single_with::<QuitButton>, RunningState::Quit),
    ]
}
