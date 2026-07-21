//! GTW-655: the dev-tools procgen load-time stepper.
//!
//! Seven tests, all against the REAL `GdtfLoadTestAppBuilder` Load flow (a live `AssetServer`
//! rooted at the workspace `assets/`, so the prefab / theme / terrain registries are populated
//! from SHIPPED content — the same real-registries proof `procgen_battle.rs` uses) and a FIXED
//! injected `BattleSeed` so the app instances draw identically. Shared setup lives in
//! [`harness`]; the tests themselves are split by concern:
//!
//! - [`regression`] — the un-engaged / disabled-plugin fingerprint pins:
//!   `normal_path_reaches_running_with_a_terrain_fingerprint` (NO stepper plugin added — mirrors
//!   a normal `cargo run`, or a `dev_tools` build where `GDTF_PROCGEN_STEPPER` is unset) and
//!   `stepper_disabled_plugin_is_indistinguishable_from_absent` (`with_enabled(false)` registers
//!   nothing, so adding it changes nothing about the normal path either).
//! - [`step_equivalence`] — the LOAD-BEARING step-equivalence tests:
//!   `stepper_engaged_path_matches_the_normal_fingerprint` drives the SAME seed ONE STAGE AT A
//!   TIME via `PendingStepCommand::request` (bypassing egui entirely — the closure never runs
//!   headlessly, bevy-traps #8) and reaches `BattleRunning` with the IDENTICAL terrain-entity
//!   count as the normal path; `stepper_engaged_path_deploys_the_same_roster` (GTW-765) drives
//!   that same stepped path and asserts the finish DEPLOYS the roster — the same non-zero count of
//!   deployed ganger entities the normal path produces, not the ZERO the pre-fix terrain-only
//!   finish left; `stage_summary_reflects_the_real_driver_at_each_stage` reads the
//!   live `StagedProcgen` resource through `stage_summary` (the panel's pure formatter,
//!   `crate::dev::procgen_stepper::summary` in `gdtf_app`) after each stage, proving it names the
//!   REAL driver state the shipped registries just produced, not a synthetic fixture.
//! - [`commands`] — the Skip / Auto app-wiring tests: `skip_drives_every_remaining_stage_in_one_request`
//!   proves ONE `StepCommand::Skip` request completes the WHOLE remaining drive, not just one
//!   stage (discriminating: a Skip that only advanced one stage would strand the app in
//!   `Generation` forever with no further command pending);
//!   `auto_run_advances_every_stage_without_a_manual_command` proves toggling `AutoRunning::set`
//!   ONCE and then only advancing the app's virtual clock (via `TimeUpdateStrategy::ManualDuration`
//!   set to the SAME per-stage pace `AutoStepTimer` repeats on) free-runs the WHOLE drive to
//!   `BattleRunning` with NO `PendingStepCommand` ever requested.
//!
//! `dev_tools`-gated like the module it pins (run with `cargo test -p gdtf_app --features
//! test-support,dev_tools --test procgen_stepper`): the whole crate compiles to an empty test
//! binary without the feature — CI's static suite never enables `dev_tools`, so this suite never
//! breaks it.
#![cfg(feature = "dev_tools")]

mod commands;
mod harness;
mod regression;
mod step_equivalence;
