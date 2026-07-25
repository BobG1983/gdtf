//! The shared real-keypress emitter (GTW-783; lifted out of [`inject`](super::inject) when
//! GTW-802 gave it a second consumer).
//!
//! One place writes a discrete key tap onto the buffered `KeyboardInput` message stream —
//! the SAME stream the windowing backend feeds. Bevy's `keyboard_input_system` folds the
//! pair into `ButtonInput<KeyCode>` (so a `just_pressed`-reading game system sees the key)
//! and Bevy's `dispatch_focused_input` triggers a `FocusedInput<KeyboardInput>` at the
//! focused entity (so a first-party widget's own key observer sees it). Nothing here forges
//! a game-level activation message; it emits the input a player's key produces and lets the
//! game's own systems react.
//!
//! Two consumers: the GTW-783 raw-input arm of [`apply_injects`](super::inject::apply_injects)
//! (an arbitrary key), and the GTW-802
//! [`drive_focus_control`](super::focus_control::drive_focus_control) activation
//! ([`activation_key_tap`], the `Enter` the focus framework treats as "activate this").

use bevy::{
    ecs::message::Messages,
    input::{
        ButtonState,
        keyboard::{Key, KeyCode, KeyboardInput, NativeKey},
    },
    prelude::*,
};

/// The key an activation emits — the SAME key the focus-nav keyboard bridge and the
/// first-party checkbox both treat as "activate the focused control".
const ACTIVATION_KEY: KeyCode = KeyCode::Enter;

/// Write a real `KeyboardInput` press+release pair for `key_code` on `window` — a clean
/// discrete tap. Inert if the stream is absent (no windowing input stack).
pub(super) fn emit_key_tap(
    key_code: KeyCode,
    window: Entity,
    events: Option<&mut Messages<KeyboardInput>>,
) {
    let Some(events) = events else {
        return;
    };
    events.write(key_message(key_code, ButtonState::Pressed, window));
    events.write(key_message(key_code, ButtonState::Released, window));
}

/// Write the ACTIVATION key tap ([`Enter`](KeyCode::Enter)) on `window` — what a QA-driven
/// "activate the focused control" emits, so the game's own keyboard bridges raise their own
/// activation signals instead of the QA layer forging one.
pub(super) fn activation_key_tap(window: Entity, events: Option<&mut Messages<KeyboardInput>>) {
    emit_key_tap(ACTIVATION_KEY, window, events);
}

/// Build a `KeyboardInput` message for `key_code` in `state` on `window`. The physical
/// `key_code` and `state` are what `keyboard_input_system` (and the game's keybind reads)
/// consume; `logical_key` is left unidentified — the QA layer names keys physically, not by
/// a layout-specific character.
const fn key_message(key_code: KeyCode, state: ButtonState, window: Entity) -> KeyboardInput {
    KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: None,
        repeat: false,
        window,
    }
}
