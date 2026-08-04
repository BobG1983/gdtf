use bevy::prelude::*;

#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct SoundEnabled(bool);

impl SoundEnabled {
    pub(in crate::states::running::options) const fn new(on: bool) -> Self {
        Self(on)
    }

    pub(in crate::states::running::options) const fn is_on(self) -> bool {
        self.0
    }
}

#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct SoundSettingChanged(SoundEnabled);

impl SoundSettingChanged {
    pub(in crate::states::running::options) const fn new(value: SoundEnabled) -> Self {
        Self(value)
    }
}

pub(in crate::states::running::options) const fn sound_value_text(
    sound: SoundEnabled,
) -> &'static str {
    if sound.is_on() { "On" } else { "Off" }
}
