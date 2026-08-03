use bevy::prelude::*;

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsScreenRoot;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsTitle;
}

crate::support_item! {
                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SoundToggle;
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::options) struct SoundToggleKnob;

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SoundValueLabel;
}

crate::support_item! {
                                            #[cfg(feature = "dev_tools")]
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ProcgenStepperToggle;
}

#[cfg(feature = "dev_tools")]
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::options) struct ProcgenStepperToggleKnob;

crate::support_item! {
                    #[cfg(feature = "dev_tools")]
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ProcgenStepperValueLabel;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ContinueButton;
}
