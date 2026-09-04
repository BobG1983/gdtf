mod plugin;
pub(in crate::states::running) use plugin::GameScenePlugin;
mod resources;

mod game_state;
crate::support_use!(game_state::GameState;);

mod setup;
pub(in crate::states::running::game) use setup::GameSetupScenePlugin;

mod hivescape;
pub(in crate::states::running::game) use hivescape::GameHiveScapeScenePlugin;

pub(crate) mod battlescape;
pub(in crate::states::running::game) use battlescape::GameBattleScapeScenePlugin;
crate::support_use!(battlescape::BattleScapeState;);
crate::support_use!(battlescape::AfterMathState;);
