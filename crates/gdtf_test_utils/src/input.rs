//! Synthetic keyboard, mouse, and UI button input for tests.

use bevy::{
    app::App,
    input::{ButtonInput, keyboard::KeyCode, mouse::MouseButton},
    prelude::{Entity, Interaction},
};

/// Press a keyboard key on the app.
pub fn press_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .get_resource_or_insert_with(ButtonInput::<KeyCode>::default)
        .press(key);
}

/// Release and clear all keyboard keys.
pub fn clear_keys(app: &mut App) {
    let mut keys = app
        .world_mut()
        .get_resource_or_insert_with(ButtonInput::<KeyCode>::default);
    keys.release_all();
    keys.clear();
}

/// Press a mouse button on the app.
pub fn press_mouse(app: &mut App, button: MouseButton) {
    app.world_mut()
        .get_resource_or_insert_with(ButtonInput::<MouseButton>::default)
        .press(button);
}

/// Press the left mouse button.
pub fn press_left(app: &mut App) {
    press_mouse(app, MouseButton::Left);
}

/// Release and clear left mouse button state.
pub fn clear_mouse(app: &mut App) {
    let mut mouse = app
        .world_mut()
        .get_resource_or_insert_with(ButtonInput::<MouseButton>::default);
    mouse.release(MouseButton::Left);
    mouse.clear();
}

/// Set a UI entity's [`Interaction`] to Pressed.
pub fn press_ui_button(app: &mut App, button: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
}
