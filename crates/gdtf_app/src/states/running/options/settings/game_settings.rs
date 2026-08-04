use bevy::prelude::*;

use super::sound::SoundEnabled;
#[cfg(feature = "dev_tools")]
use super::stepper::ProcgenStepperEnabled;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct GameSettings {
    pub(in crate::states::running::options) sound:           SoundEnabled,
    #[cfg(feature = "dev_tools")]
    pub(in crate::states::running::options) procgen_stepper: ProcgenStepperEnabled,
}

impl GameSettings {
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
    fn default() -> Self {
        Self {
            sound: SoundEnabled::new(true),
            #[cfg(feature = "dev_tools")]
            procgen_stepper: ProcgenStepperEnabled::OFF,
        }
    }
}
