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

// The GTW-575 scene-scaffold helpers: the four stamped system shapes (enter/exit
// loggers, completion-marker insert, scoped-resource remove, move-on transition)
// every scene plugin below registers instead of hand-stamped one-file systems.
mod scaffold;

mod plugin;
crate::support_use!(plugin::ScenesPlugin;);

mod init;
pub(in crate::states) use init::InitScenePlugin;

mod intro;
pub(in crate::states) use intro::IntroScenePlugin;

mod load;
pub(in crate::states) use load::LoadScenePlugin;
// GTW-590: extend the GTW-582 re-export ladder one rung further — the
// `dev::capture` loudness pins live OUTSIDE `states`, and must drive their
// emission asserts through the ONE shared, poison-proof log-capture scaffold
// (a second CaptureLayer copy would lose the process-global-default race and
// capture nothing).
#[cfg(test)]
pub(crate) use load::hot_reload_test_support;
// The resolved authored battlefield resource (GTW-205 / E10.3), re-exported here so
// it is nameable from OUTSIDE `states` — `test_support` widens it to `pub` for the
// AC7 real-asset harness (`crate::states::LoadedSituation`), and the GTW-655 DEV
// procgen stepper (`crate::dev::procgen_stepper::drive`) names it `pub(crate)` in a
// `dev_tools` binary build to resolve the same authored situation the normal
// `request_battle_setup` path reads. (Within `states`, E10.5 still reaches the
// resource directly via `load::LoadedSituation`, which is why that site names no
// climb at all.) GTW-655 retired the `procgen_viz` scene — the last consumer of this
// root climb outside `test-support`/`dev_tools` — so the climb is now gated on
// `any(test-support, dev_tools)`: with BOTH off (a plain release/CI build), nothing
// in-crate names `crate::states::LoadedSituation`, and an unconditional `support_use!`
// would leave a genuinely unused `pub(crate) use` (caught by `cargo check` without
// features — the same class `cargo dbuild` catches for the binary).
#[cfg(any(feature = "test-support", feature = "dev_tools"))]
crate::support_use!(load::LoadedSituation;);
// The bespoke headless Load-fallback seed (GTW-629), re-exported here so it is
// nameable from OUTSIDE `states` — the GTW-223 DEV auto-battle affordance
// (`crate::dev::auto_battle`) registers it on `Startup`, and `test_support`
// widens it to `pub` for the load-suite gate seed.
crate::support_use!(load::seed_load_fallbacks;);

// `pub(crate)` so the crate-root test-support ledger can name the panel `test_support`
// submodules under it (GTW-569 one-hop ledger — markers no longer climb through here).
pub(crate) mod running;
pub(in crate::states) use running::RunningScenePlugin;
// The four sub-state enums are co-located with the modules they govern (GTW-321);
// each climbs through its scene `mod.rs` to `running`, and these unconditional
// `support_use!`s carry them the last hop to `crate::states::<Enum>` — the same
// import depth they had in the old flat `states/` folder, so `test_support` and
// every `use crate::states::…` keeps compiling unchanged. Panel/test markers do NOT
// climb through here — the crate-root test-support ledger names each panel's own
// `test_support` submodule directly (GTW-569 one-hop ledger).
crate::support_use!(running::RunningState;);
crate::support_use!(running::GameState;);
crate::support_use!(running::BattleScapeState;);
crate::support_use!(running::AfterMathState;);

mod teardown;
pub(in crate::states) use teardown::TeardownScenePlugin;
