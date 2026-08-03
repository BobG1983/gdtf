mod acts;
mod components;
mod plugin;
mod registrar;
mod seam;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::ContextualPanelPlugin;

#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::{
        acts::{
            enter_emplacement::EnterEmplacementButton, execute::ExecuteButton,
            exit_emplacement::ExitEmplacementButton, melee::MeleeButton, open_door::OpenDoorButton,
            shove::ShoveButton, stabilize::StabilizeButton, throw_grenade::ThrowGrenadeButton,
        },
        components::ContextualPanelRoot,
    };
}
