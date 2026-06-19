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
// Test-support-only re-export of the explicit end-signal marker (GTW-236), gated so the
// binary build stays `unused`/`unreachable_pub`-clean. Carries `BattleRunningComplete` up
// toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use!(battlescape::BattleRunningComplete;);
// Test-support-only re-export of the action-bar's per-act button markers (GTW-228),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the menu-marker
// re-export chain precedent). Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    battlescape::{
        AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
        ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
        StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
        StanceStandingButton,
    };
}
// Test-support-only re-export of the GTW-278 shared stat-block markers + the GTW-274
// inspect-panel markers, gated so the binary build is `unused`/`unreachable_pub`-clean (the
// action-bar per-act-marker re-export chain precedent). Carries the markers up toward
// `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    battlescape::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
        StatFaction, StatHpBar, StatName, StatPortrait, StatStance, StatTuBar, StatWoundLine,
        StatWoundList, StatWoundsPips, portrait_index_for_name,
    };
}
// Test-support-only re-export of the GTW-275 weapon-panel markers + the layout-overhaul
// bottom-bar root, gated so the binary build is `unused`/`unreachable_pub`-clean. Carries the
// markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    battlescape::{
        AimLabel, AimPanel, BottomBarRoot, CombinedWeaponPanel, ReloadButton, WeaponContent,
        WeaponImage, WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText,
        WeaponPanelRoot,
    };
}
