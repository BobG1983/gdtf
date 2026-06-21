mod plugin;
mod systems;
pub(in crate::states) use plugin::RunningScenePlugin;
mod resources;

// The running sub-state enum lives in the folder it governs (GTW-321); this
// `support_use!` carries it up toward `crate::states::RunningState`.
mod running_state;
crate::support_use!(running_state::RunningState;);

mod menu;
pub(in crate::states::running) use menu::MenuScenePlugin;
// Test-support-only re-export (see menu/mod.rs); gated so the binary build is
// warning-clean. (GTW-145)
#[cfg(feature = "test-support")]
crate::support_use! {
    menu::{BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton};
}

mod game;
pub(in crate::states::running) use game::GameScenePlugin;
// The game + battlescape + aftermath sub-state enums climb from `game` toward
// `crate::states::{GameState, BattleScapeState, AfterMathState}` (GTW-321 chain).
crate::support_use!(game::GameState;);
crate::support_use!(game::BattleScapeState;);
crate::support_use!(game::AfterMathState;);
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
        ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
        StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
        StanceStandingButton,
    };
}
// Test-support-only re-export of the GTW-278 stat-block markers + the GTW-274 inspect-panel
// markers, gated so the binary build is `unused`/`unreachable_pub`-clean. Carries the
// markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
        StatFaction, StatHpBar, StatHpLabel, StatName, StatPortrait, StatStance, StatTuBar,
        StatTuLabel, StatWoundLine, StatWoundList, StatWoundsPips, portrait_index_for_name,
    };
}
// Test-support-only re-export of the GTW-275 weapon-panel markers + the layout-overhaul
// bottom-bar root, gated so the binary build is `unused`/`unreachable_pub`-clean. Carries the
// markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{
        AimLabel, AimPanel, BottomBarRoot, CombinedWeaponPanel, ReloadButton, WeaponContent,
        WeaponImage, WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText,
        WeaponPanelRoot,
    };
}
// Test-support-only re-export of the GTW-294 contextual-panel markers, gated so the binary build
// is `unused`/`unreachable_pub`-clean. Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{
        ContextualPanelRoot, ExecuteButton, OpenDoorButton, StabilizeButton,
    };
}
// Test-support-only re-export of the GTW-328 combat-log markers, gated so the binary build is
// `unused`/`unreachable_pub`-clean. Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    game::{CombatLogLine, CombatLogRoot};
}

mod options;
pub(in crate::states::running) use options::OptionsScenePlugin;

mod quit;
pub(in crate::states::running) use quit::QuitScenePlugin;
