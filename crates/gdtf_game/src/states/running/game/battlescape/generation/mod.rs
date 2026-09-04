mod plugin;
pub(in crate::states::running) use plugin::GameBattleScapeGenerationScenePlugin;
mod resources;
#[cfg(feature = "mcp")]
pub(crate) use resources::GenerationComplete;
#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::resources::GenerationComplete;
}

// drive through the SAME deploy + finding-conversion logic `request_battle_setup` uses
pub(crate) mod battle_sim;

pub(crate) mod loading_screen;
