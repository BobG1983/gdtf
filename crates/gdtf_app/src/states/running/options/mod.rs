mod components;
mod plugin;
mod settings;
mod systems;

pub(in crate::states::running) use plugin::OptionsScenePlugin;

#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::components::{
        ContinueButton, OptionsScreenRoot, OptionsTitle, SoundToggle, SoundValueLabel,
    };
    #[cfg(feature = "dev_tools")]
    pub use super::components::{ProcgenStepperToggle, ProcgenStepperValueLabel};
}
