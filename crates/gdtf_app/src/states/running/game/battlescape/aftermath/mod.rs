mod plugin;
pub(in crate::states::running) use plugin::GameBattleScapeAfterMathScenePlugin;
mod resources;

mod aftermath_state;
crate::support_use!(aftermath_state::AfterMathState;);

mod animate_in;
pub(in crate::states::running::game::battlescape::aftermath) use animate_in::GameBattleScapeAfterMathAnimateInScenePlugin;

mod display_aftermath;
pub(in crate::states::running::game::battlescape::aftermath) use display_aftermath::GameBattleScapeAfterMathDisplayAftermathScenePlugin;

mod animate_out;
pub(in crate::states::running::game::battlescape::aftermath) use animate_out::GameBattleScapeAfterMathAnimateOutScenePlugin;
