mod spawn;
pub(in crate::states::running::game::battlescape::weapon_panel) use spawn::{
    despawn_weapon_panel, spawn_weapon_panel,
};

mod update;
pub(in crate::states::running::game::battlescape::weapon_panel) use update::update_weapon_panel;

mod actions;
pub(in crate::states::running::game::battlescape::weapon_panel) use actions::{
    reload_button_focus_activated, reload_button_pressed,
};

mod fit;
pub(in crate::states::running::game::battlescape::weapon_panel) use fit::fit_weapon_panel;
