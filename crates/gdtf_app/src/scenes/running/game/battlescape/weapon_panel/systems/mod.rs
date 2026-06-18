mod spawn;
pub(in crate::scenes::running::game::battlescape::weapon_panel) use spawn::{
    despawn_weapon_panel, spawn_weapon_panel,
};

mod update;
pub(in crate::scenes::running::game::battlescape::weapon_panel) use update::update_weapon_panel;

mod actions;
pub(in crate::scenes::running::game::battlescape::weapon_panel) use actions::reload_button_pressed;
