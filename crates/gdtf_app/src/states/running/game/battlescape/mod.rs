mod plugin;
mod systems;
pub(in crate::states::running) use plugin::GameBattleScapeScenePlugin;
mod resources;

mod battlescape_state;
crate::support_use!(battlescape_state::BattleScapeState;);

pub(crate) mod generation;
pub(in crate::states::running::game::battlescape) use generation::GameBattleScapeGenerationScenePlugin;

mod animate_in;
pub(in crate::states::running::game::battlescape) use animate_in::GameBattleScapeAnimateInScenePlugin;

pub(crate) mod battle_running;
pub(in crate::states::running::game::battlescape) use battle_running::GameBattleScapeBattleRunningScenePlugin;

mod animate_out;
pub(in crate::states::running::game::battlescape) use animate_out::GameBattleScapeAnimateOutScenePlugin;

mod aftermath;
pub(in crate::states::running::game::battlescape) use aftermath::GameBattleScapeAfterMathScenePlugin;
crate::support_use!(aftermath::AfterMathState;);

pub(crate) mod action_bar;
pub(in crate::states::running::game::battlescape) use action_bar::GameBattleScapeActionBarScenePlugin;

pub(crate) mod stat_block;

pub(crate) mod status_panel;
pub(in crate::states::running::game::battlescape) use status_panel::GameBattleScapeStatusPanelScenePlugin;

pub(crate) mod inspect_panel;
pub(in crate::states::running::game::battlescape) use inspect_panel::GameBattleScapeInspectPanelScenePlugin;

mod bottom_bar;
pub(in crate::states::running::game::battlescape) use bottom_bar::GameBattleScapeBottomBarScenePlugin;
crate::support_use!(bottom_bar::BottomBarRoot;);

pub(crate) mod weapon_panel;
pub(in crate::states::running::game::battlescape) use weapon_panel::GameBattleScapeWeaponPanelScenePlugin;

pub(crate) mod select_cycle;
pub(in crate::states::running::game::battlescape) use select_cycle::GameBattleScapeSelectCycleScenePlugin;

pub(crate) mod combat_log;
pub(in crate::states::running::game::battlescape) use combat_log::GameBattleScapeCombatLogScenePlugin;

pub(crate) mod contextual_panel;
pub(in crate::states::running::game::battlescape) use contextual_panel::ContextualPanelPlugin;

mod focus_nav;
pub(in crate::states::running::game::battlescape) use focus_nav::GameBattleScapeFocusNavScenePlugin;
