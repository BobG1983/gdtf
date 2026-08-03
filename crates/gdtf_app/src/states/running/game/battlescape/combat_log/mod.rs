mod components;
mod plugin;
mod systems;
mod tuning;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeCombatLogScenePlugin;

#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{CombatLogLine, CombatLogRoot};
}
