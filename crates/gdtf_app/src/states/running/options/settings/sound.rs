//! The sound setting — the GTW-637 pilot's ONE player-facing setting.
//!
//! The pilot ships this one real setting — whether in-game sound is enabled — so the
//! adapter tax of bridging a first-party `gdtf_ui` widget to a typed settings intent is
//! sized against a genuine value, not a placeholder. The value is a named newtype
//! ([`SoundEnabled`], not a bare `bool`) and the toggle emits a typed newtype
//! [`Message`] ([`SoundSettingChanged`], not a bare `bool`/enum) — the "typed settings
//! intent" clause of the pilot.
//!
//! Flow: the first-party [`bevy_ui_widgets::Checkbox`](bevy::ui_widgets::Checkbox) the
//! screen spawns emits its native [`ValueChange<bool>`](bevy::ui_widgets::ValueChange)
//! on activation; the screen's observer
//! ([`sound_activated`](super::super::systems::sound_activated)) maps the activated
//! toggle (identified by its [`SoundToggle`](super::super::components::SoundToggle)
//! marker) to a [`SoundSettingChanged`], and
//! [`apply_sound_setting`](super::super::systems::apply_sound_setting) folds it into
//! [`GameSettings`](super::GameSettings). The widget never learns what the toggle MEANS
//! — the screen owns that mapping.

use bevy::prelude::*;

/// Whether in-game sound is enabled — the single player-facing setting the pilot
/// Options screen toggles.
///
/// A named newtype over `bool` (no-bare-types rule): call sites read
/// `SoundEnabled::is_on()` / construct `SoundEnabled::new(true)` rather than pass
/// an opaque flag. It derefs to the inner `bool` for the rare read that wants it.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct SoundEnabled(bool);

impl SoundEnabled {
    /// Wrap a raw on/off flag as a typed sound-enabled value.
    pub(in crate::states::running::options) const fn new(on: bool) -> Self {
        Self(on)
    }

    /// Whether sound is enabled.
    pub(in crate::states::running::options) const fn is_on(self) -> bool {
        self.0
    }
}

/// A typed settings intent: the sound-enabled setting changed to a new value.
///
/// A newtype [`Message`] over [`SoundEnabled`] (bevy-traps rule 4; no bare
/// `bool`/enum on the wire) written by the screen's toggle adapter and folded into
/// [`GameSettings`](super::GameSettings) by the apply system. Keeping the intent a typed
/// message — rather than mutating the resource inline in the adapter — matches the
/// codebase's input→intent→apply shape and lets the toggle bridge and the settings fold
/// be wired and tested independently.
#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct SoundSettingChanged(SoundEnabled);

impl SoundSettingChanged {
    /// Wrap the new [`SoundEnabled`] value as a settings-changed intent.
    pub(in crate::states::running::options) const fn new(value: SoundEnabled) -> Self {
        Self(value)
    }
}

/// The value-readout text for a [`SoundEnabled`] state ("On" / "Off").
///
/// Takes the typed [`SoundEnabled`] rather than a bare `bool` (no-bare-types rule);
/// shared by the spawn seed and the change-driven label sync so the two never drift.
pub(in crate::states::running::options) const fn sound_value_text(
    sound: SoundEnabled,
) -> &'static str {
    if sound.is_on() { "On" } else { "Off" }
}
