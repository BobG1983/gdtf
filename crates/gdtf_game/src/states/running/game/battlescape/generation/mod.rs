mod plugin;
pub(in crate::states::running) use plugin::GameBattleScapeGenerationScenePlugin;
mod resources;
#[cfg(feature = "mcp")]
pub(crate) use resources::GenerationComplete;
#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::resources::GenerationComplete;
}

// The one route that generates a battle: begin, advance, finish.
pub(crate) mod battle_sim;

pub(crate) mod loading_screen;
