mod spawn;
pub(in crate::scenes::running::game::battlescape::action_bar) use spawn::{
    despawn_action_bar, spawn_action_bar,
};

mod actions;
pub(in crate::scenes::running::game::battlescape::action_bar) use actions::action_bar_button_intents;

mod flee;
pub(in crate::scenes::running::game::battlescape::action_bar) use flee::flee_button_pressed;

mod aim_active;
pub(in crate::scenes::running::game::battlescape::action_bar) use aim_active::sync_aim_button_active;

mod stance_active;
pub(in crate::scenes::running::game::battlescape::action_bar) use stance_active::sync_stance_buttons_active;

mod nowrap;
pub(in crate::scenes::running::game::battlescape::action_bar) use nowrap::nowrap_control_labels;

pub(in crate::scenes::running::game::battlescape::action_bar) mod mode_panel;
pub(in crate::scenes::running::game::battlescape::action_bar) use mode_panel::{
    mode_button_pressed, rebuild_mode_buttons, sync_mode_buttons_active,
};

mod stance_panel;
// Re-exported so the action-bar `mod.rs` can carry the relocated-control spawn constructors one
// hop wider to the sibling weapon-panel module (GTW-298). The press → intent + active-mark
// systems stay in this module; only the spawn constructors are shared.
pub(in crate::scenes::running::game::battlescape) use aim_active::spawn_aim_button;
pub(in crate::scenes::running::game::battlescape) use mode_panel::spawn_mode_panel;
pub(in crate::scenes::running::game::battlescape) use stance_panel::spawn_stance_panel;
