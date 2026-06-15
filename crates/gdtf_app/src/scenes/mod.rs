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
// The resolved authored battlefield resource (GTW-205 / E10.3), re-exported here
// ONLY for the test-support surface so the AC7 real-asset harness can name it
// (`test_support` re-exports `crate::scenes::LoadedSituation`). In the binary
// build it stays `pub(crate)` at its definition and is reached directly within the
// crate (E10.5), so this re-export is test-support-gated to keep the binary
// `unused`-clean — mirroring the button-marker re-export below.
#[cfg(feature = "test-support")]
crate::support_use!(load::LoadedSituation;);

mod running;
pub(in crate::scenes) use running::RunningScenePlugin;
// Test-support-only re-export (see menu/mod.rs); gated so the binary build is
// warning-clean. (GTW-145)
#[cfg(feature = "test-support")]
crate::support_use! {
    running::{BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton};
}

mod teardown;
pub(in crate::scenes) use teardown::TeardownScenePlugin;
