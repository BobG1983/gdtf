//! Options scene: root, title, sound toggle, continue back to menu.
use bevy::{
    ecs::{entity::Entity, system::RunSystemOnce},
    prelude::{Commands, Text},
    state::state::{NextState, State},
    ui::{Checked, Interaction},
    ui_widgets::ValueChange,
};
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_game::test_support::{
    AppState, ContinueButton, OptionsScreenRoot, OptionsTitle, RunningState, SoundToggle,
    SoundValueLabel,
};
use gdtf_ui::{focus_nav::FocusActivated, theme::default_theme};

fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn pending_state(app: &bevy::app::App) -> Option<RunningState> {
    match app.world().resource::<NextState<RunningState>>() {
        NextState::Pending(state) | NextState::PendingIfNeq(state) => Some(*state),
        NextState::Unchanged => None,
    }
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

fn set_interaction(app: &mut bevy::app::App, entity: Entity, state: Interaction) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity) {
        *interaction = state;
    }
}

fn sound_value_text(app: &mut bevy::app::App) -> Option<String> {
    let entity = single_with::<SoundValueLabel>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|text| text.as_str().to_owned())
}

fn options_app() -> bevy::app::App {
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .starting_in(AppState::Running)
            .build();
    app.world_mut().insert_resource(default_theme());
    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Options)
    });
    app
}

#[test]
fn options_screen_spawns_root_title_and_widgets() {
    let mut app = options_app();
    assert_eq!(
        running_state(&app),
        Some(RunningState::Options),
        "precondition: the app must rest in RunningState::Options",
    );
    assert!(
        single_with::<OptionsScreenRoot>(&mut app).is_some(),
        "the Options screen must spawn an OptionsScreenRoot",
    );
    assert!(
        single_with::<OptionsTitle>(&mut app).is_some(),
        "and an OptionsTitle heading",
    );
    assert!(
        single_with::<SoundToggle>(&mut app).is_some(),
        "and the sound-toggle Checkbox (the one first-party widget)",
    );
    assert!(
        single_with::<ContinueButton>(&mut app).is_some(),
        "and a Continue button",
    );
}

#[test]
fn dev_only_stepper_toggle_is_present_exactly_under_dev_tools() {
    let mut app = options_app();
    let mut checkboxes = app
        .world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<bevy::ui_widgets::Checkbox>>();
    let count = checkboxes.iter(app.world()).count();
    #[cfg(feature = "dev_tools")]
    assert_eq!(
        count, 2,
        "a dev_tools build must show TWO setting toggles: sound + the dev procgen stepper",
    );
    #[cfg(not(feature = "dev_tools"))]
    assert_eq!(
        count, 1,
        "a non-dev_tools build must show exactly ONE setting toggle (sound) — no dev control",
    );
}

#[test]
fn continue_mouse_press_returns_to_menu() {
    let mut app = options_app();
    let button = single_with::<ContinueButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    set_interaction(&mut app, button, Interaction::Pressed);
    app.update();
    assert_eq!(
        pending_state(&app),
        Some(RunningState::Menu),
        "a mouse press on Continue must request RunningState::Menu (back to the Main Menu), \
         not RunningState::Game",
    );
}

#[test]
fn continue_focus_activation_returns_to_menu() {
    let mut app = options_app();
    let button = single_with::<ContinueButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut().write_message(FocusActivated::new(button));
    app.update();
    assert_eq!(
        pending_state(&app),
        Some(RunningState::Menu),
        "a focus activation on Continue must request RunningState::Menu (back to the Main Menu), \
         not RunningState::Game",
    );
}

fn activate_sound_checkbox(app: &mut bevy::app::App, checkbox: Entity) {
    let checked = app.world().get::<Checked>(checkbox).is_some();
    let value = !checked;
    let triggered = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.trigger(ValueChange {
                source: checkbox,
                value,
                is_final: true,
            });
        });
    assert!(
        triggered.is_ok(),
        "the one-shot ValueChange trigger system must run",
    );
}

#[test]
fn sound_toggle_flip_updates_value_label() {
    let mut app = options_app();
    assert_eq!(
        sound_value_text(&mut app).as_deref(),
        Some("On"),
        "sound defaults On, so the readout starts On",
    );

    let checkbox = single_with::<SoundToggle>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    assert!(
        app.world().get::<Checked>(checkbox).is_some(),
        "precondition: the sound checkbox starts Checked (sound On)",
    );

    activate_sound_checkbox(&mut app, checkbox);
    loop {
        app.update();
        if sound_value_text(&mut app).as_deref() == Some("Off") {
            break;
        }
    }
    assert!(
        app.world().get::<Checked>(checkbox).is_none(),
        "the first-party checkbox_self_update observer must clear Checked when toggled Off",
    );
}
