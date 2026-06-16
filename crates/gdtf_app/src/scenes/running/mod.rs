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
// Test-support-only re-export of the explicit end-signal marker (GTW-236), gated so the
// binary build stays `unused`/`unreachable_pub`-clean. Carries `BattleRunningComplete` up
// toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use!(game::BattleRunningComplete;);
// Test-support-only re-export of the action-bar's per-act button markers (GTW-228),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the menu-marker
// re-export chain precedent). Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{
        AimToggleButton, EndTurnButton, FireModeSelectButton, FleeButton, LevelDownButton,
        LevelUpButton, ReloadButton, StanceCycleButton,
    };
}
// Test-support-only re-export of the status-panel's per-line text markers (GTW-252),
// gated so the binary build is `unused`/`unreachable_pub`-clean. Carries the markers up
// toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{HpText, IdentityText, LifeText, StanceText, TuText};
}

mod options;
pub(in crate::scenes::running) use options::OptionsScenePlugin;

mod quit;
pub(in crate::scenes::running) use quit::QuitScenePlugin;
