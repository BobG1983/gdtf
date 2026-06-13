mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::GameBattleScapeAfterMathScenePlugin;
mod resources;

mod animate_in;
pub(in crate::scenes::running::game::battlescape::aftermath) use animate_in::GameBattleScapeAfterMathAnimateInScenePlugin;

mod display_after;
pub(in crate::scenes::running::game::battlescape::aftermath) use display_after::GameBattleScapeAfterMathDisplayAfterScenePlugin;

mod animate_out;
pub(in crate::scenes::running::game::battlescape::aftermath) use animate_out::GameBattleScapeAfterMathAnimateOutScenePlugin;
