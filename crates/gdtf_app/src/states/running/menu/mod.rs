mod plugin;
mod systems;
pub(in crate::states::running) use plugin::MenuScenePlugin;

mod start_battle;
pub(crate) use start_battle::StartBattleRequested;
pub(in crate::states::running::menu) use start_battle::apply_start_battle;

mod components;
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::{
        components::{BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton},
        start_battle::StartBattleRequested,
    };
}
