//! The stepper's ENGAGEMENT gate: the [`ProcgenStepperActive`] marker resource, and the
//! run condition `battle_sim::plugin` reads off its absence.
//!
//! Engagement is a RUNTIME state, not a build-time one (GTW-868). Under `dev_tools` the
//! stepper's drive systems are always registered; whether they take over a battle
//! generation depends solely on whether [`ProcgenStepperActive`] is in the world at the
//! moment `BattleScapeState::Generation` is entered. The Options screen's dev-only
//! procgen-stepper toggle inserts and removes it (see
//! `crate::states::running::options`), and
//! [`ProcgenStepperPlugin::with_enabled`](super::plugin::ProcgenStepperPlugin::with_enabled)
//! seeds it at plugin build for a headless test that wants the engaged path without
//! driving the UI.
//!
//! The marker (rather than two independent reads of whatever decides engagement) is what
//! keeps the two registration sites — this module's plugin and `battle_sim::plugin`, which
//! have no direct call dependency — consistent: both look at the one resource.

use bevy::prelude::{Res, Resource};

crate::support_item! {
    /// Marks the procgen stepper ENGAGED: the next `BattleScapeState::Generation` entry is
    /// driven by the stepper instead of running battle setup straight through.
    ///
    /// Present exactly while the dev-only procgen-stepper setting is on — the Options screen's
    /// toggle inserts and removes it (GTW-868) — or from plugin build when a test forced
    /// engagement with
    /// [`ProcgenStepperPlugin::with_enabled(true)`](super::plugin::ProcgenStepperPlugin::with_enabled).
    /// Its mere presence is what `battle_setup_runs_directly` (this module's crate-private run
    /// condition) gates `request_battle_setup` off on; a build without `dev_tools` never
    /// defines it at all.
    ///
    /// Engagement is read at `OnEnter(BattleScapeState::Generation)`, so a flip takes effect
    /// for the NEXT battle generation — flipping it mid-generation is not supported.
    ///
    /// `pub` under `test-support` (the GTW-868 integration test asserts the toggle inserts
    /// and removes it), `pub(crate)` otherwise.
    #[derive(Resource, Debug, Default, Clone, Copy)]
    struct ProcgenStepperActive;
}

/// The run condition `battle_sim::plugin` gates `request_battle_setup` on: `true` (run
/// setup directly, the normal path) whenever [`ProcgenStepperActive`] is absent.
///
/// When the stepper IS engaged, `request_battle_setup` is skipped for every
/// `BattleScapeState::Generation` entry and [`super::drive::engage_stepper`] drives the
/// battle instead — see `battle_sim::plugin`'s `cfg(feature = "dev_tools")` wiring for the
/// exact run-condition swap and why it is behavior-preserving when the stepper is off.
/// It is evaluated per frame, so inserting or removing the marker at runtime swaps the two
/// paths for the next generation entry.
#[must_use]
pub(crate) const fn battle_setup_runs_directly(active: Option<Res<ProcgenStepperActive>>) -> bool {
    active.is_none()
}
