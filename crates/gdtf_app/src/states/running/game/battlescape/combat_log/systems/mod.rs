mod spawn;
pub(in crate::states::running::game::battlescape::combat_log) use spawn::{
    despawn_combat_log, spawn_combat_log,
};

mod update;
pub(in crate::states::running::game::battlescape::combat_log) use update::{
    animate_combat_log_height, fade_combat_log_lines, slide_combat_log_lines, update_combat_log,
};
