mod components;
mod plugin;
mod systems;

pub(in crate::states::running::game::battlescape) use components::{
    BOTTOM_BAR_H_VH, BOTTOM_BAR_PAD_X_VW, BOTTOM_BAR_PAD_Y_VH,
};
pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeBottomBarScenePlugin;
pub(in crate::states::running::game::battlescape) use systems::{
    despawn_bottom_bar, spawn_bottom_bar,
};

crate::support_use!(components::BottomBarRoot;);
