//! The persisted, app-wide [`GameSettings`] resource the Options screen edits.

use bevy::prelude::*;

use super::sound::SoundEnabled;
#[cfg(feature = "dev_tools")]
use super::stepper::ProcgenStepperEnabled;

/// The persisted, app-wide game settings the Options screen edits.
///
/// Inserted once at plugin build (not scene-scoped) so a setting survives leaving
/// and re-entering the Options screen — the screen reads it to seed each control's
/// initial state and writes it back through that setting's typed intent message.
///
/// The DEV-ONLY procgen-stepper field exists only under the `dev_tools` Cargo feature
/// (GTW-868): a non-`dev_tools` build has no such setting, no control for it, and no
/// wiring — the screen is exactly what it was.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct GameSettings {
    /// Whether in-game sound is enabled.
    pub(in crate::states::running::options) sound:           SoundEnabled,
    /// Whether the DEV-ONLY procgen load-time stepper is engaged for the next battle
    /// generation (`dev_tools` builds only).
    #[cfg(feature = "dev_tools")]
    pub(in crate::states::running::options) procgen_stepper: ProcgenStepperEnabled,
}

impl GameSettings {
    /// The same settings with `sound` replaced — a builder-style override so a caller can
    /// pick ONE setting without naming (or knowing about) the cfg-gated dev field, which a
    /// struct-update literal cannot do in both build configurations.
    #[cfg(test)]
    #[must_use]
    pub(in crate::states::running::options) const fn with_sound(
        mut self,
        sound: SoundEnabled,
    ) -> Self {
        self.sound = sound;
        self
    }
}

impl Default for GameSettings {
    /// Sound on by default — the resting state a fresh install shows. The dev-only
    /// procgen stepper defaults OFF, so a plain `cargo drun` reaches a battle with no
    /// overlay and no pause until a developer turns it on (GTW-868).
    fn default() -> Self {
        Self {
            sound: SoundEnabled::new(true),
            #[cfg(feature = "dev_tools")]
            procgen_stepper: ProcgenStepperEnabled::OFF,
        }
    }
}
