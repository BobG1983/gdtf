//! Shared headless input synthesis (GTW-576) — the raw keyboard / mouse /
//! `Interaction` press primitives the per-file test copies collapsed onto.
//!
//! These are the PRIMITIVES; suite-specific composites (hover-then-click chains, a
//! "press the marked button and settle" helper) stay in their suites and COMPOSE
//! these. Under `MinimalPlugins` no `ui_focus_system` ticks the input edges, so a
//! pressed key/button stays just-pressed until [`clear_keys`] / [`clear_mouse`].

use bevy::{
    app::App,
    input::{ButtonInput, keyboard::KeyCode, mouse::MouseButton},
    prelude::{Entity, Interaction},
};

/// Presses (just-pressed edge) `key` on the `ButtonInput<KeyCode>` buffer
/// (inserting a default buffer if the harness never registered one).
pub fn press_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .get_resource_or_insert_with(ButtonInput::<KeyCode>::default)
        .press(key);
}

/// Releases all held keys + clears the keyboard edges so the NEXT [`press_key`] is
/// a fresh just-pressed. A still-HELD key makes a re-`press` a no-op (no new
/// just-pressed edge), so a looped press must `release_all()` THEN `clear()`.
pub fn clear_keys(app: &mut App) {
    let mut keys = app
        .world_mut()
        .get_resource_or_insert_with(ButtonInput::<KeyCode>::default);
    keys.release_all();
    keys.clear();
}

/// Presses (just-pressed edge) `button` on the `ButtonInput<MouseButton>` buffer
/// (inserting a default buffer if the harness never registered one).
pub fn press_mouse(app: &mut App, button: MouseButton) {
    app.world_mut()
        .get_resource_or_insert_with(ButtonInput::<MouseButton>::default)
        .press(button);
}

/// Presses (just-pressed edge) the left mouse button.
pub fn press_left(app: &mut App) {
    press_mouse(app, MouseButton::Left);
}

/// Releases + clears the mouse edges so a later press is a fresh just-pressed.
pub fn clear_mouse(app: &mut App) {
    let mut mouse = app
        .world_mut()
        .get_resource_or_insert_with(ButtonInput::<MouseButton>::default);
    mouse.release(MouseButton::Left);
    mouse.clear();
}

/// Synthesizes a fresh mouse press on a UI `button` entity by setting its
/// [`Interaction`] to [`Interaction::Pressed`] (the swap `ui_focus_system` drives
/// for a real click). The direct write marks the component `Changed` this update,
/// so a `Changed<Interaction>` press query fires. No `update()` is run — the caller
/// owns the tick (some suites must run only `Update` so `ui_focus_system` cannot
/// clobber the injected press back to `None` first).
pub fn press_ui_button(app: &mut App, button: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
}
