mod spawn;
pub(in crate::scenes::running::game::battlescape::inspect_panel) use spawn::{
    despawn_inspect_panel, spawn_inspect_panel,
};

mod update;
pub(in crate::scenes::running::game::battlescape::inspect_panel) use update::update_inspect_panel;
