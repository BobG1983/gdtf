mod components;
mod plugin;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeWeaponPanelScenePlugin;

#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{
        AimLabel, AimPanel, CombinedWeaponPanel, ReloadButton, WeaponContent, WeaponImage,
        WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
    };
}
