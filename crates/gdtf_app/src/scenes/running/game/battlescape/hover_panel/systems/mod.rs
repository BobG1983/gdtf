mod spawn;
pub(in crate::scenes::running::game::battlescape::hover_panel) use spawn::{
    despawn_hover_panel, spawn_hover_panel,
};

mod update;
pub(in crate::scenes::running::game::battlescape::hover_panel) use update::update_hover_panel;
