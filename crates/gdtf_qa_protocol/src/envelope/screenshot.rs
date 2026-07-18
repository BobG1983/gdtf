//! [`ScreenshotResult`] + [`ScreenshotPathNet`] — the reply to a screenshot request
//! (GTW-734), plus [`ScreenshotAfterResult`] — the reply to a deferred
//! [`ScreenshotAfter`](crate::envelope::QaRequest::ScreenshotAfter) (GTW-749).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

use super::receipt::RejectReason;

/// The on-disk **path** a screenshot was saved to — constrained under
/// `target/qa_screenshots/` by the game side (GTW-694).
///
/// A path newtype over `String` (no-bare-types), serde-transparent. `Clone`-not-`Copy`.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScreenshotPathNet(String);

impl ScreenshotPathNet {
    /// Build a screenshot path from its saved location.
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }
}

/// The reply to a [`TakeScreenshot`](crate::envelope::QaRequest::TakeScreenshot) —
/// whether the capture landed on disk.
///
/// The GTW-694 screenshot flow only replies once the file `exists()` on disk (or the
/// frame budget elapses): [`Saved`](Self::Saved) carries the constrained
/// [`ScreenshotPathNet`], [`TimedOut`](Self::TimedOut) is the capture-timeout outcome.
/// An independent serde enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScreenshotResult {
    /// The screenshot was captured and written to the given path.
    Saved(ScreenshotPathNet),
    /// The capture did not land on disk within the frame budget.
    TimedOut,
}

/// The reply to a [`ScreenshotAfter`](crate::envelope::QaRequest::ScreenshotAfter) —
/// the embedded intent's wire-layer outcome, folded together with the deferred
/// capture's outcome (GTW-749).
///
/// The embedded intent is injected through the same path a bare
/// [`Inject`](crate::envelope::QaRequest::Inject) uses, so it can be
/// [`Rejected`](Self::Rejected) for exactly the same reasons (`NoBattle`,
/// `NotOffered`, …) — a rejection means NO capture was ever attempted, never a shot of
/// the wrong moment. Once the intent queues, the capture proceeds exactly like a
/// [`TakeScreenshot`](crate::envelope::QaRequest::TakeScreenshot): [`Saved`](Self::Saved)
/// once the PNG verifiably lands on disk, [`TimedOut`](Self::TimedOut) once the poll
/// budget elapses. An independent serde enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScreenshotAfterResult {
    /// The embedded intent was rejected at the wire layer — no capture was attempted.
    Rejected(RejectReason),
    /// The intent queued and the deferred capture landed at the given path.
    Saved(ScreenshotPathNet),
    /// The intent queued but the deferred capture did not land within the frame budget.
    TimedOut,
}
