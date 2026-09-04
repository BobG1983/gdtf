use bevy::prelude::*;

crate::support_item! {
    /// Root node of the options screen.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsScreenRoot;
}

crate::support_item! {
    /// The options screen heading text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsTitle;
}

crate::support_item! {
    /// The sound on/off toggle track.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SoundToggle;
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::options) struct SoundToggleKnob;

crate::support_item! {
    /// The text showing whether sound is on or off.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SoundValueLabel;
}

crate::support_item! {
    /// The procgen stepper on/off toggle track.
    #[cfg(feature = "dev_tools")]
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ProcgenStepperToggle;
}

#[cfg(feature = "dev_tools")]
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::options) struct ProcgenStepperToggleKnob;

crate::support_item! {
    /// The text showing whether the procgen stepper is on or off.
    #[cfg(feature = "dev_tools")]
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ProcgenStepperValueLabel;
}

crate::support_item! {
    /// Button that leaves the options screen.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ContinueButton;
}
