//! The per-call riders a `Run` may carry — [`RunOptions`], [`AwaitBudget`],
//! [`CaptureRider`] (GTW-939).
//!
//! The TYPES land here. The BEHAVIOUR lands later: `gdtf_qa_command` (A3) answers any
//! non-default [`RunOptions`] with
//! [`UnavailableCode::NotBuilt`](crate::command::UnavailableCode::NotBuilt), and C2
//! builds the two riders for real.

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use crate::ids::ShotName;

/// How long a caller will let a host WAIT for a command to become admissible, in whole
/// seconds.
///
/// Private-inner newtype over `u64` (no-bare-types), serde-transparent. A budget of zero
/// means "admit it now or refuse it now" — the same behaviour as omitting the rider, said
/// explicitly.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AwaitBudget(u64);

impl AwaitBudget {
    /// Build an await budget from its whole-second count.
    #[must_use]
    pub const fn new(seconds: u64) -> Self {
        Self(seconds)
    }
}

/// A request to capture the screen once the command has run.
///
/// The rider that turns any command into "do this, then show me" without every command
/// growing a capture argument: the host runs the command, captures, and hands the PNG back
/// as a [`ReplyAttachment`](crate::command::ReplyAttachment) on the same outcome.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaptureRider {
    /// The capture's file stem, or `None` for a host-chosen name.
    pub name: Option<ShotName>,
}

impl CaptureRider {
    /// Build a capture rider from its optional file stem.
    #[must_use]
    pub const fn new(name: Option<ShotName>) -> Self {
        Self { name }
    }
}

/// The riders one [`Run`](crate::envelope::QaRequest::Run) may carry — how long to wait
/// for admission, and whether to capture the screen afterwards.
///
/// [`Default`] is both riders absent: run the command now, capture nothing. That default
/// is the value a host compares against to decide whether a call asked for machinery it
/// has not built yet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunOptions {
    /// How long to keep re-testing admission before giving up, or `None` to decide once.
    pub await_ready: Option<AwaitBudget>,
    /// The capture to take once the command has run, or `None` for no capture.
    pub capture:     Option<CaptureRider>,
}

impl RunOptions {
    /// Build the rider set from an await budget and a capture request.
    #[must_use]
    pub const fn new(await_ready: Option<AwaitBudget>, capture: Option<CaptureRider>) -> Self {
        Self {
            await_ready,
            capture,
        }
    }

    /// Whether both riders are absent — a plain call asking for no extra machinery.
    ///
    /// The predicate a host with stubbed riders tests: anything else is answered
    /// [`NotBuilt`](crate::command::UnavailableCode::NotBuilt) rather than silently run
    /// without the rider the caller asked for.
    #[must_use]
    pub const fn is_plain(&self) -> bool {
        self.await_ready.is_none() && self.capture.is_none()
    }
}
