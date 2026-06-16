//! Scene flow for the GDTF app.
//!
//! This module owns ONLY scene transitions and per-scene setup/teardown: the
//! `AppState` (and sub-state) machine wiring via `OnEnter` / `OnExit`, tracking
//! when a scene is complete, and moving on to the next state. Everything else —
//! gameplay, the combat sim, presentation, UI — lives in other crates/modules.
//! A scene here just enters, does its setup, hands off, and tears down.

mod plugin;
crate::support_use!(plugin::ScenesPlugin;);

mod init;
pub(in crate::scenes) use init::InitScenePlugin;

mod intro;
pub(in crate::scenes) use intro::IntroScenePlugin;

mod load;
pub(in crate::scenes) use load::LoadScenePlugin;
// The resolved authored battlefield resource (GTW-205 / E10.3), re-exported here so
// it is nameable from OUTSIDE `scenes` — `test_support` widens it to `pub` for the
// AC7 real-asset harness (`crate::scenes::LoadedSituation`), and the GTW-223 DEV
// auto-battle affordance (`crate::app::auto_battle`) names it `pub(crate)` in the
// binary build to seed a default battlefield. (Within `scenes`, E10.5 still reaches
// the resource directly via `load::LoadedSituation`.) Unconditional `support_use!`,
// so it tracks `support_item` visibility in lockstep (the re-export chain caveat).
crate::support_use!(load::LoadedSituation;);

mod running;
pub(in crate::scenes) use running::RunningScenePlugin;
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
        AimToggleButton, EndTurnButton, FireModeSelectButton, LevelDownButton, LevelUpButton,
        ReloadButton, StanceCycleButton,
    };
}

mod teardown;
pub(in crate::scenes) use teardown::TeardownScenePlugin;
