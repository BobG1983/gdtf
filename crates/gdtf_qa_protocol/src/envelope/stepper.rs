//! [`StepperCommandNet`] + [`AutoRunNet`] — the DEV procgen-stepper drive command, and
//! [`StepperReceipt`] — the reply to a stepper-control request (GTW-766).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// Whether the DEV procgen stepper's **Auto** free-run is turned on.
///
/// A private-inner newtype over `bool` (no-bare-types: `true` = free-run on, `false` = off),
/// serde-transparent — the [`AimNet`](crate::intent::AimNet) precedent. Carried by
/// [`StepperCommandNet::Auto`] so a client sets Auto to an ABSOLUTE state (never a flip),
/// exactly as the panel's two Start/Stop buttons do — an assignment made twice ends at the
/// same value, so a repeat is idempotent.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AutoRunNet(bool);

impl AutoRunNet {
    /// Build an Auto free-run setting — `true` to start free-running, `false` to stop.
    #[must_use]
    pub const fn new(running: bool) -> Self {
        Self(running)
    }
}

/// A single DEV procgen-stepper drive command a QA client sends — the wire mirror of the
/// stepper panel's three egui controls (GTW-766).
///
/// Carried by [`QaRequest::StepperControl`](crate::envelope::QaRequest::StepperControl). The
/// game side maps each variant onto the SAME idempotent latch a panel button writes:
/// [`Next`](Self::Next) / [`Skip`](Self::Skip) onto the step-command latch, [`Auto`](Self::Auto)
/// onto the Auto-run latch. An independent serde enum — never a leak of the game's own
/// `StepCommand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepperCommandNet {
    /// Advance exactly one stage (the panel's Next button).
    Next,
    /// Set the Auto free-run toggle to an absolute state (the panel's Start/Stop Auto
    /// buttons).
    Auto {
        /// Whether Auto free-run is turned on.
        running: AutoRunNet,
    },
    /// Drive every remaining stage to completion immediately (the panel's Skip button).
    Skip,
}

/// The reply to a [`StepperControl`](crate::envelope::QaRequest::StepperControl) — whether
/// the command reached the stepper's latch (GTW-766).
///
/// [`Latched`](Self::Latched) means the command was written into the same idempotent latch a
/// panel button writes (the game-side `PendingStepCommand` / `AutoRunning`), to be applied by
/// the drive on its next pass. [`Inactive`](Self::Inactive) is the fail-closed reply when the
/// stepper's latch is absent — distinct from the route-time
/// [`StepperInactive`](crate::envelope::QaError::StepperInactive) error, it is the
/// dispatch-time defense-in-depth answer, unreachable in practice because the router already
/// rejects a `StepperControl` with no live drive before it is ever queued. An independent
/// serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepperReceipt {
    /// The command was written into the stepper's latch.
    Latched,
    /// No stepper drive is in flight — nothing was latched (fail-closed).
    Inactive,
}
