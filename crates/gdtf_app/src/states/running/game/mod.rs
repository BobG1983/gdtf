mod plugin;
mod systems;
pub(in crate::states::running) use plugin::GameScenePlugin;
mod resources;

// The game sub-state enum lives in the folder it governs (GTW-321); this
// `support_use!` carries it up toward `crate::states::GameState`.
mod game_state;
crate::support_use!(game_state::GameState;);

mod setup;
pub(in crate::states::running::game) use setup::GameSetupScenePlugin;

mod hivescape;
pub(in crate::states::running::game) use hivescape::GameHiveScapeScenePlugin;

mod battlescape;
pub(in crate::states::running::game) use battlescape::GameBattleScapeScenePlugin;
// The battlescape + aftermath sub-state enums climb from `battlescape` toward
// `crate::states::{BattleScapeState, AfterMathState}` (GTW-321 co-location chain).
crate::support_use!(battlescape::BattleScapeState;);
crate::support_use!(battlescape::AfterMathState;);
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
        StabilityBar, StatFaction, StatHpBar, StatHpLabel, StatInjuryLine, StatInjuryList,
        StatName, StatPortrait, StatStance, StatTuBar, StatTuLabel, StatWoundLine, StatWoundList,
        StatWoundsPips, portrait_index_for_name,
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
// Test-support-only re-export of the GTW-458 selection-cycle markers (the cluster root + the
// Next / Prev buttons), gated so the binary build is `unused`/`unreachable_pub`-clean. Carries
// the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    battlescape::{SelectCycleRoot, SelectNextButton, SelectPrevButton};
}
// Test-support-only re-export of the GTW-294 contextual-panel markers (Execute / Stabilize /
// Open Door buttons + the panel root), gated so the binary build is `unused`/`unreachable_pub`-
// clean. Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    battlescape::{
        ContextualPanelRoot, ExecuteButton, LoadingScreenRoot, MeleeButton, OpenDoorButton,
        ShoveButton, StabilizeButton,
    };
}
// Test-support-only re-export of the GTW-328 combat-log markers (the log root + per-line marker),
// gated so the binary build is `unused`/`unreachable_pub`-clean. Carries the markers up toward
// `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    battlescape::{CombatLogLine, CombatLogRoot};
}
