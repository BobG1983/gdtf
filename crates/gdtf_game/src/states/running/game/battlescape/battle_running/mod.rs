mod plugin;
mod systems;
pub(in crate::states::running) use plugin::GameBattleScapeBattleRunningScenePlugin;
mod resources;
#[cfg(feature = "mcp")]
pub(crate) use resources::BattleRunningComplete;
pub(crate) use resources::insert_battle_running_complete;
#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::resources::BattleRunningComplete;
}
