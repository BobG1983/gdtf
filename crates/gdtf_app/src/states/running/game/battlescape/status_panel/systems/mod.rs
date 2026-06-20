mod spawn;
pub(in crate::states::running::game::battlescape::status_panel) use spawn::{
    despawn_status_panel, spawn_status_panel,
};

mod update;
pub(in crate::states::running::game::battlescape::status_panel) use update::update_status_panel;
