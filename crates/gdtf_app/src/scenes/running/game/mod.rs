mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::GameScenePlugin;
mod resources;

mod setup;
pub(in crate::scenes::running::game) use setup::GameSetupScenePlugin;

mod hivescape;
pub(in crate::scenes::running::game) use hivescape::GameHiveScapeScenePlugin;

mod battlescape;
pub(in crate::scenes::running::game) use battlescape::GameBattleScapeScenePlugin;
