//! The DEV-ONLY procgen-stepper setting (GTW-868) — present only in `dev_tools` builds.
//!
//! The procgen load-time stepper PAUSES battle generation between the sim's staged
//! assemble / fill / emit stages, waiting for Next / Auto / Skip. `dev_tools` is folded
//! into every dev alias (`dclippy` / `dtest` / `dbuild` / `drun`), so "engaged whenever
//! `dev_tools` is compiled in" would stall every `cargo drun` at an overlay before every
//! battle and hang the test suite. Availability follows `dev_tools`; ENGAGEMENT is this
//! setting, which defaults to OFF.
//!
//! Same shape as every other setting: a named newtype value ([`ProcgenStepperEnabled`],
//! not a bare `bool`), a typed newtype [`Message`] intent
//! ([`ProcgenStepperSettingChanged`]), a [`GameSettings`](super::GameSettings) field, and
//! a readout formatter ([`stepper_value_text`]). Flipping it inserts / removes
//! `ProcgenStepperActive`
//! ([`sync_stepper_engagement`](super::super::systems::sync_stepper_engagement)), which
//! the stepper's `OnEnter(Generation)` run condition reads — so a flip takes effect for
//! the NEXT battle generation.

use bevy::prelude::*;

/// Whether the DEV-ONLY procgen load-time stepper is engaged.
///
/// A named newtype over `bool` (no-bare-types rule) with a named [`OFF`](Self::OFF)
/// resting value, so the default reads as a decision rather than a bare `false`.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct ProcgenStepperEnabled(bool);

impl ProcgenStepperEnabled {
    /// The default: the stepper is NOT engaged, so a battle generates straight through.
    pub(in crate::states::running::options) const OFF: Self = Self(false);

    /// Wrap a raw on/off flag as a typed procgen-stepper-enabled value.
    pub(in crate::states::running::options) const fn new(on: bool) -> Self {
        Self(on)
    }

    /// Whether the stepper is engaged.
    pub(in crate::states::running::options) const fn is_on(self) -> bool {
        self.0
    }
}

/// A typed settings intent: the procgen-stepper setting changed to a new value.
///
/// A newtype [`Message`] over [`ProcgenStepperEnabled`] (bevy-traps rule 4; no bare
/// `bool` on the wire), written by the toggle's native `ValueChange` observer and folded
/// into [`GameSettings`](super::GameSettings) by the apply system, which the engagement
/// system then follows.
#[derive(Message, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::options) struct ProcgenStepperSettingChanged(ProcgenStepperEnabled);

impl ProcgenStepperSettingChanged {
    /// Wrap the new [`ProcgenStepperEnabled`] value as a settings-changed intent.
    pub(in crate::states::running::options) const fn new(value: ProcgenStepperEnabled) -> Self {
        Self(value)
    }
}

/// The value-readout text for a [`ProcgenStepperEnabled`] state ("On" / "Off").
///
/// Takes the typed value rather than a bare `bool` (no-bare-types rule); shared by the
/// spawn seed and the change-driven label sync so the two never drift.
pub(in crate::states::running::options) const fn stepper_value_text(
    stepper: ProcgenStepperEnabled,
) -> &'static str {
    if stepper.is_on() { "On" } else { "Off" }
}
