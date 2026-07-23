//! The stepper's activation gate: an env-var reader for the plugin's own `from_env`
//! constructor, and a build-time-inserted [`ProcgenStepperActive`] marker
//! `battle_sim::plugin`'s `request_battle_setup` run condition gates on.
//!
//! The marker (not a second independent env-var read) is what keeps the two
//! registration sites — this plugin and `battle_sim::plugin`, which have no direct call
//! dependency — consistent, AND lets a test force the engaged path via
//! [`ProcgenStepperPlugin::with_enabled`](super::plugin::ProcgenStepperPlugin::with_enabled)
//! without mutating the process-global `GDTF_PROCGEN_STEPPER` env var (which would race
//! against any OTHER test thread reading it). [`ProcgenStepperPlugin::build`] inserts
//! [`ProcgenStepperActive`] ONCE, at plugin-build time (before any system ever runs) —
//! there is no per-frame ordering hazard between the two read sites, unlike a per-`Generation`
//! `OnEnter` insert would have.

use bevy::prelude::{Res, Resource};

/// The `GDTF_PROCGEN_STEPPER` environment variable that opts a `dev_tools` dev build into
/// the load-time procgen stepper.
const PROCGEN_STEPPER_ENV: &str = "GDTF_PROCGEN_STEPPER";

crate::support_item! {
    /// Whether the DEV-ONLY procgen stepper is enabled for this process.
    ///
    /// Reads the `PROCGEN_STEPPER_ENV` (`GDTF_PROCGEN_STEPPER`) environment variable and
    /// treats `1` / `true` / `yes` / `on` (case-insensitive, trimmed) as enabled; anything
    /// else — including the variable being unset or empty — is disabled. Mirrors the house
    /// recognised-truthy convention every other `GDTF_*` gate in this crate uses (e.g.
    /// `net_qa`'s `net_qa_enabled`); kept as its own parser rather than a shared helper
    /// (short of the rule of three — each gate's doc explains why it stays intra-module).
    ///
    /// Pure (no `World`, no side effects), so `super::plugin`'s `from_env` gate can call it
    /// with no side effects: a normal `cargo run` (even a `dev_tools` build) leaves the var
    /// unset, this returns `false`, and a battle loads exactly as it does without
    /// `dev_tools`. `pub` under `test-support` (the integration test unit-checks the parser
    /// directly), `pub(crate)` otherwise.
    #[must_use]
    fn stepper_enabled() -> bool {
        std::env::var(PROCGEN_STEPPER_ENV).is_ok_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    }
}

/// Marks the procgen stepper ACTIVE for this whole process run.
///
/// Inserted by [`ProcgenStepperPlugin::build`](super::plugin::ProcgenStepperPlugin::build)
/// exactly when it proceeds (its own `enabled` gate held) — a ONE-TIME, build-time insert,
/// never per-`Generation`-span. Its mere presence is what
/// [`battle_setup_runs_directly`] gates `request_battle_setup` off on; a build without
/// `dev_tools`, or a `dev_tools` build whose gate did not hold, never inserts it.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub(crate) struct ProcgenStepperActive;

/// The run condition `battle_sim::plugin` gates `request_battle_setup` on: `true` (run
/// setup directly, the normal path) whenever [`ProcgenStepperActive`] is absent.
///
/// When the stepper IS active, `request_battle_setup` is skipped for every
/// `BattleScapeState::Generation` entry and [`super::drive::engage_stepper`] drives the
/// battle instead — see `battle_sim::plugin`'s `cfg(feature = "dev_tools")` wiring for the
/// exact run-condition swap and why it is behavior-preserving when the stepper is off.
#[must_use]
pub(crate) const fn battle_setup_runs_directly(active: Option<Res<ProcgenStepperActive>>) -> bool {
    active.is_none()
}
