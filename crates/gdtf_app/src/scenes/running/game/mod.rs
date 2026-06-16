mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::GameScenePlugin;
mod resources;

mod setup;
pub(in crate::scenes::running::game) use setup::GameSetupScenePlugin;

mod hivescape;
pub(in crate::scenes::running::game) use hivescape::GameHiveScapeScenePlugin;

mod battlescape;
pub(in crate::scenes::running::game) use battlescape::GameBattleScapeScenePlugin;
// Test-support-only re-export of the action-bar's per-act button markers (GTW-228),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the menu-marker
// re-export chain precedent). Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    battlescape::{
        AimToggleButton, EndTurnButton, FireModeSelectButton, LevelDownButton, LevelUpButton,
        ReloadButton, StanceCycleButton,
    };
}
