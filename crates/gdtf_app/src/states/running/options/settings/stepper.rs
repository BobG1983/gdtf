use bevy::prelude::*;

#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct ProcgenStepperEnabled(bool);

impl ProcgenStepperEnabled {
        pub(in crate::states::running::options) const OFF: Self = Self(false);

        pub(in crate::states::running::options) const fn new(on: bool) -> Self {
        Self(on)
    }

        pub(in crate::states::running::options) const fn is_on(self) -> bool {
        self.0
    }
}

#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct ProcgenStepperSettingChanged(ProcgenStepperEnabled);

impl ProcgenStepperSettingChanged {
        pub(in crate::states::running::options) const fn new(value: ProcgenStepperEnabled) -> Self {
        Self(value)
    }
}

pub(in crate::states::running::options) const fn stepper_value_text(
    stepper: ProcgenStepperEnabled,
) -> &'static str {
    if stepper.is_on() { "On" } else { "Off" }
}
