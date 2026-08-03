mod plugin;
pub(in crate::states::running) use plugin::GameBattleScapeGenerationScenePlugin;
mod resources;

// drive through the SAME deploy + finding-conversion logic `request_battle_setup` uses
pub(crate) mod battle_sim;

pub(crate) mod loading_screen;
