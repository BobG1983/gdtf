mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::GameBattleScapeScenePlugin;
mod resources;

mod generation;
pub(in crate::scenes::running::game::battlescape) use generation::GameBattleScapeGenerationScenePlugin;

mod animate_in;
pub(in crate::scenes::running::game::battlescape) use animate_in::GameBattleScapeAnimateInScenePlugin;

mod battle_running;
pub(in crate::scenes::running::game::battlescape) use battle_running::GameBattleScapeBattleRunningScenePlugin;

mod animate_out;
pub(in crate::scenes::running::game::battlescape) use animate_out::GameBattleScapeAnimateOutScenePlugin;

mod aftermath;
pub(in crate::scenes::running::game::battlescape) use aftermath::GameBattleScapeAfterMathScenePlugin;
