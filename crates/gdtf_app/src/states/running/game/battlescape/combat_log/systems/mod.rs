mod append;
mod forward;
mod sources;
mod spawn;
mod update;

pub(in crate::states::running::game::battlescape::combat_log) use append::append_combat_log;
pub(in crate::states::running::game::battlescape::combat_log) use forward::{
    CombatLogSourceAppExt, CombatLogSystems, forward_turn_started,
};
pub(in crate::states::running::game::battlescape::combat_log) use spawn::{
    despawn_combat_log, spawn_combat_log,
};
pub(in crate::states::running::game::battlescape::combat_log) use update::{
    animate_combat_log_height, fade_combat_log_lines, slide_combat_log_lines,
};
