mod plugin;
mod systems;
pub(in crate::scenes) use plugin::RunningScenePlugin;
mod resources;

mod menu;
pub(in crate::scenes::running) use menu::MenuScenePlugin;
// Test-support-only re-export (see menu/mod.rs); gated so the binary build is
// warning-clean. (GTW-145)
#[cfg(feature = "test-support")]
crate::support_use! {
    menu::{BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton};
}

mod game;
pub(in crate::scenes::running) use game::GameScenePlugin;
// Test-support-only re-export of the action-bar's per-act button markers (GTW-228),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the menu-marker
// re-export chain precedent). Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{
        AimToggleButton, EndTurnButton, FireModeSelectButton, LevelDownButton, LevelUpButton,
        ReloadButton, StanceCycleButton,
    };
}

mod options;
pub(in crate::scenes::running) use options::OptionsScenePlugin;

mod quit;
pub(in crate::scenes::running) use quit::QuitScenePlugin;
