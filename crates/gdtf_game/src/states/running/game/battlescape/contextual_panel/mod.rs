mod acts;
mod components;
mod plugin;
pub(crate) mod registrar;
pub(crate) mod seam;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::ContextualPanelPlugin;
#[cfg(feature = "mcp")]
pub(crate) use seam::ContextualOffer;

#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::{
        acts::{
            enter_emplacement::EnterEmplacementButton, execute::ExecuteButton,
            exit_emplacement::ExitEmplacementButton, melee::MeleeButton, open_door::OpenDoorButton,
            shove::ShoveButton, stabilize::StabilizeButton, throw_grenade::ThrowGrenadeButton,
        },
        components::ContextualPanelRoot,
        registrar::ContextualPanelSystems,
    };
}
