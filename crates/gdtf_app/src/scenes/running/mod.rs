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
        AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
        ModeBurstButton, ModeFullButton, ModePanelRoot, ModeSingleButton,
        StanceKneelingButton, StancePanelRoot, StanceProneButton, StanceStandingButton,
    };
}
// Test-support-only re-export of the GTW-278 stat-block markers + the GTW-274 hover-panel
// markers, gated so the binary build is `unused`/`unreachable_pub`-clean. Carries the
// markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{
        HoverObjectBar, HoverObjectBlock, HoverObjectHardness, HoverObjectHeight,
        HoverObjectProtection, HoverObjectText, HoverPanelRoot, HoverStatBlockHost, StatFaction,
        StatHpBar, StatName, StatPortrait, StatStance, StatTuBar, StatWoundLine, StatWoundList,
        StatWoundsPips, portrait_index_for_name,
    };
}
// Test-support-only re-export of the GTW-275 weapon-panel markers, gated so the binary build
// is `unused`/`unreachable_pub`-clean. Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{
        ReloadButton, WeaponContent, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
    };
}

mod options;
pub(in crate::scenes::running) use options::OptionsScenePlugin;

mod quit;
pub(in crate::scenes::running) use quit::QuitScenePlugin;
