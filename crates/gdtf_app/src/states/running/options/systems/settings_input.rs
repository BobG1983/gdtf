//! The ONE widget adapter (GTW-637, INPUT clause): the first-party
//! [`bevy_ui_widgets::Checkbox`](bevy::ui_widgets::Checkbox) sound toggle bridged to
//! the typed settings model through the widget's OWN native activation event.
//!
//! `bevy_ui_widgets::Checkbox` is a headless first-party widget: when it is
//! activated it emits a [`ValueChange<bool>`](bevy::ui_widgets::ValueChange) carrying
//! the new checked state. It never learns what the toggle MEANS. This module is the
//! adapter that gives it meaning, in the codebase's input→intent→apply→view shape,
//! with the INPUT step driven by the widget's native `On<ValueChange<bool>>` observer
//! contract (the ADR 0003 amendment's first-party-widget input path) — no hand-rolled
//! bridge sits between the widget and the intent:
//!
//! 1. [`sound_activated`] — the `On<ValueChange<bool>>` observer that, for the sound
//!    toggle (identified by its [`SoundToggle`] marker), reads the native
//!    [`ValueChange`](bevy::ui_widgets::ValueChange)'s new value and writes the typed
//!    newtype [`SoundSettingChanged`] intent (no bare `bool` crosses the boundary).
//!    The checkbox's own first-party observers turn the player's input into that
//!    `ValueChange`: pressing `Enter` / `Space` while the toggle holds
//!    [`InputFocus`](bevy::input_focus::InputFocus) fires
//!    `bevy_ui_widgets`' `checkbox_on_key_input` (registered by the `CheckboxPlugin`
//!    in `DefaultPlugins`), which emits the `ValueChange` this observer reads. Focus
//!    reaches the toggle through the project's existing `focus_nav` directional
//!    navigation — no `ui_picking` backend is involved, so no pointer-arbitration
//!    change touches the battle HUD.
//! 2. [`apply_sound_setting`] — folds [`SoundSettingChanged`] into the persisted
//!    [`GameSettings`] resource.
//! 3. [`sync_sound_value_label`] — on a [`GameSettings`] change, MUTATES the value
//!    readout text in place (the project's mutate-not-respawn UI convention).
//!
//! The checkbox's own [`Checked`](bevy::ui::Checked) state is kept in step by the
//! first-party [`checkbox_self_update`](bevy::ui_widgets::checkbox_self_update)
//! observer, which the scene plugin registers — so each activation reads the current
//! state and toggles to its complement.

use bevy::{prelude::*, ui_widgets::ValueChange};

use crate::states::running::options::{
    components::{SoundToggle, SoundValueLabel},
    settings::{GameSettings, SoundEnabled, SoundSettingChanged, sound_value_text},
};

/// The `On<ValueChange<bool>>` observer that writes the typed [`SoundSettingChanged`]
/// intent when the sound-toggle [`Checkbox`](bevy::ui_widgets::Checkbox) is activated.
///
/// A global observer (registered once at plugin build): it fires for every
/// [`ValueChange<bool>`](bevy::ui_widgets::ValueChange), so it filters on the
/// [`SoundToggle`] marker (a `ValueChange` from any other future widget is a no-op for
/// it) and reads the native event's new value ([`ValueChange::value`](bevy::ui_widgets::ValueChange::value)
/// — `true` when the checkbox becomes checked, i.e. sound on). This is the pilot's
/// first-party-widget INPUT path writing a typed newtype settings intent, driven
/// straight off the widget's native activation event with no intervening hand-rolled
/// bridge.
pub(in crate::states::running::options) fn sound_activated(
    change: On<ValueChange<bool>>,
    toggles: Query<(), With<SoundToggle>>,
    mut changed: MessageWriter<SoundSettingChanged>,
) {
    if toggles.contains(change.source) {
        changed.write(SoundSettingChanged::new(SoundEnabled::new(change.value)));
    }
}

/// Folds a [`SoundSettingChanged`] intent into the persisted [`GameSettings`].
///
/// Applies the last-written value (a later intent this frame wins) to
/// [`GameSettings::sound`], mutating the resource in place so the change persists
/// across leaving and re-entering the screen. Gated
/// `run_if(in_state(RunningState::Options))` by the scene plugin.
pub(in crate::states::running::options) fn apply_sound_setting(
    mut changed: MessageReader<SoundSettingChanged>,
    mut settings: ResMut<GameSettings>,
) {
    for change in changed.read() {
        settings.sound = **change;
    }
}

/// Mutates the sound value readout text when [`GameSettings`] changes.
///
/// Change-driven (`Res::is_changed`): on the frame the setting flips it rewrites
/// the [`SoundValueLabel`] text ("On" / "Off") in place — no despawn/respawn (the
/// mutate-in-place convention). Runs while the screen is up; the `Changed` guard
/// keeps it from touching the label on steady frames.
pub(in crate::states::running::options) fn sync_sound_value_label(
    settings: Res<GameSettings>,
    mut labels: Query<&mut Text, With<SoundValueLabel>>,
) {
    if !settings.is_changed() {
        return;
    }
    let text = sound_value_text(settings.sound);
    for mut label in &mut labels {
        *label = Text::new(text);
    }
}
