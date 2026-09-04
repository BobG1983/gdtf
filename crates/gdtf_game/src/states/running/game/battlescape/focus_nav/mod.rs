mod bridge;
mod outline;
mod plugin;
#[cfg(test)]
mod test;
mod topology;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeFocusNavScenePlugin;
pub(in crate::states::running::game::battlescape) use topology::{
    ACTION_BAR_NAV_BASE, CONTEXTUAL_NAV_BASE, WEAPON_NAV_BASE,
};
