mod components;
mod plugin;
mod shadow;
mod systems;

#[cfg(test)]
mod test;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeInspectPanelScenePlugin;

#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::components::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
    };
}
