mod components;
mod plugin;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeActionBarScenePlugin;
pub(in crate::states::running::game::battlescape) use systems::{
    spawn_aim_button, spawn_mode_panel, spawn_stance_panel,
};

#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::components::{
        AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
        ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
        StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
        StanceStandingButton,
    };
}
