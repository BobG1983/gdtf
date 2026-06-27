//! State flow for the GDTF app.
//!
//! This module owns ONLY state transitions and per-state setup/teardown: the
//! [`AppState`] (and sub-state) machine wiring via `OnEnter` / `OnExit`, tracking
//! when a state is complete, and moving on to the next state. Everything else —
//! gameplay, the combat sim, presentation, UI — lives in other crates/modules.
//! A state here just enters, does its setup, hands off, and tears down.
//!
//! Each state enum is **co-located** with the module it governs: [`AppState`]
//! at this root, and each sub-state in the folder it gates ([`RunningState`] in
//! `running/`, [`GameState`] in `running/game/`, [`BattleScapeState`] in
//! `running/game/battlescape/`, [`AfterMathState`] in
//! `running/game/battlescape/aftermath/`), re-exported back up to this root so
//! `crate::states::<Enum>` names every one at the same depth.

mod app_state;
crate::support_use!(app_state::AppState;);

mod plugin;
crate::support_use!(plugin::ScenesPlugin;);

mod init;
pub(in crate::states) use init::InitScenePlugin;

mod intro;
pub(in crate::states) use intro::IntroScenePlugin;

mod load;
pub(in crate::states) use load::LoadScenePlugin;
// The resolved authored battlefield resource (GTW-205 / E10.3), re-exported here so
// it is nameable from OUTSIDE `states` — `test_support` widens it to `pub` for the
// AC7 real-asset harness (`crate::states::LoadedSituation`), and the GTW-223 DEV
// auto-battle affordance (`crate::app::auto_battle`) names it `pub(crate)` in the
// binary build to seed a default battlefield. (Within `states`, E10.5 still reaches
// the resource directly via `load::LoadedSituation`.) Unconditional `support_use!`,
// so it tracks `support_item` visibility in lockstep (the re-export chain caveat).
crate::support_use!(load::LoadedSituation;);

mod running;
pub(in crate::states) use running::RunningScenePlugin;
// The four sub-state enums are co-located with the modules they govern (GTW-321);
// each climbs through its scene `mod.rs` to `running`, and these unconditional
// `support_use!`s carry them the last hop to `crate::states::<Enum>` — the same
// import depth they had in the old flat `states/` folder, so `test_support` and
// every `use crate::states::…` keeps compiling unchanged.
crate::support_use!(running::RunningState;);
crate::support_use!(running::GameState;);
crate::support_use!(running::BattleScapeState;);
crate::support_use!(running::AfterMathState;);
// Test-support-only re-export of the explicit end-signal marker (GTW-236), gated so the
// binary build stays `unused`/`unreachable_pub`-clean. The reworked `state_walk` /
// `battle_running_driver` tests name it through `crate::test_support` to insert it (standing
// in for the not-yet-wired victory/flee end condition). The final hop before
// `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use!(running::BattleRunningComplete;);
// Test-support-only re-export (see menu/mod.rs); gated so the binary build is
// warning-clean. (GTW-145)
#[cfg(feature = "test-support")]
crate::support_use! {
    running::{BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton};
}
// Test-support-only re-export of the action-bar's per-act button markers (GTW-228),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the menu-marker
// re-export chain precedent). The final hop before `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    running::{
        AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
        ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
        StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
        StanceStandingButton,
    };
}
// Test-support-only re-export of the GTW-278 stat-block markers + the GTW-274 inspect-panel
// markers, gated so the binary build is `unused`/`unreachable_pub`-clean (the action-bar
// per-act-marker re-export chain precedent). The final hop before `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    running::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
        StabilityBar, StatFaction, StatHpBar, StatHpLabel, StatInjuryLine, StatInjuryList,
        StatName, StatPortrait, StatStance, StatTuBar, StatTuLabel, StatWoundLine, StatWoundList,
        StatWoundsPips, portrait_index_for_name,
    };
}
// Test-support-only re-export of the GTW-275 weapon-panel markers + the layout-overhaul
// bottom-bar root, gated so the binary build is `unused`/`unreachable_pub`-clean. The final
// hop before `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    running::{
        AimLabel, AimPanel, BottomBarRoot, CombinedWeaponPanel, ReloadButton, WeaponContent,
        WeaponImage, WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText,
        WeaponPanelRoot,
    };
}
// Test-support-only re-export of the GTW-294 contextual-panel markers, gated so the binary build
// is `unused`/`unreachable_pub`-clean. The final hop before `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    running::{
        ContextualPanelRoot, ExecuteButton, LoadingScreenRoot, OpenDoorButton, StabilizeButton,
    };
}
// Test-support-only re-export of the GTW-328 combat-log markers (the log root + per-line marker),
// gated so the binary build is `unused`/`unreachable_pub`-clean. The final hop before
// `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    running::{CombatLogLine, CombatLogRoot};
}
mod teardown;
pub(in crate::states) use teardown::TeardownScenePlugin;
