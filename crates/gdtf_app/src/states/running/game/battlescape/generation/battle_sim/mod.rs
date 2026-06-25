//! The battle-sim integration sub-module of the Generation scene: the
//! [`BattleSimPlugin`] that drives `gdtf_battle_sim` into the running app
//! (E10.5 / GTW-207).

mod plugin;
pub(in crate::states::running::game::battlescape::generation) use plugin::BattleSimPlugin;

mod seed;
mod systems;
