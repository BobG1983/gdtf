mod plugin;
mod systems;
pub(in crate::states::running) use plugin::GameBattleScapeAfterMathScenePlugin;
mod resources;

// The aftermath sub-state enum lives in the folder it governs (GTW-321); this
// `support_use!` carries it up toward `crate::states::AfterMathState`.
mod aftermath_state;
crate::support_use!(aftermath_state::AfterMathState;);

mod animate_in;
pub(in crate::states::running::game::battlescape::aftermath) use animate_in::GameBattleScapeAfterMathAnimateInScenePlugin;

mod display_aftermath;
pub(in crate::states::running::game::battlescape::aftermath) use display_aftermath::GameBattleScapeAfterMathDisplayAftermathScenePlugin;

mod animate_out;
pub(in crate::states::running::game::battlescape::aftermath) use animate_out::GameBattleScapeAfterMathAnimateOutScenePlugin;
