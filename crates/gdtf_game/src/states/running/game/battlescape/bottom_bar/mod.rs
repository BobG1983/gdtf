mod components;
mod plugin;
mod slots;
mod systems;

pub(in crate::states::running::game::battlescape) use components::{
    BOTTOM_BAR_H_VH, BOTTOM_BAR_PAD_Y_VH,
};
pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeBottomBarScenePlugin;
pub(in crate::states::running::game::battlescape) use slots::BottomBarSlots;
pub(in crate::states::running::game::battlescape) use systems::{
    despawn_bottom_bar, spawn_bottom_bar,
};

crate::support_use!(components::BottomBarRoot;);
