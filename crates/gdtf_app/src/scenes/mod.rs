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

mod running;
pub(in crate::scenes) use running::RunningScenePlugin;
crate::support_use! {
    running::{BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton};
}

mod teardown;
pub(in crate::scenes) use teardown::TeardownScenePlugin;
