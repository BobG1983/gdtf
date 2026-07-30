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

/// Why a capture was refused before it was ever spawned: what the capture was about to
/// read, against what the camera drawing the UI is actually rendering into (GTW-922).
///
/// A message newtype over `String` (no-bare-types), serde-transparent. It is a MESSAGE
/// rather than a structured pair because the host holds the only types that can name the
/// two render targets (`bevy_camera::RenderTarget`), and the protocol crate must not
/// depend on the renderer to carry a diagnosis.
///
/// Why the outcome exists at all: an offscreen capture of an image that no camera renders
/// into produces a PNG of untouched texture memory — a fully black frame that looks like a
/// successful capture. GTW-918 shipped exactly that state and GTW-922 found it live. A
/// refusal naming the mismatch is the only reply that cannot be mistaken for a screenshot
/// of the app.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CaptureAimNet(String);

impl CaptureAimNet {
    /// Build the refusal message from the host's description of the mismatch.
    #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }
}

/// The reply to a [`TakeScreenshot`](crate::envelope::QaRequest::TakeScreenshot) —
/// whether the capture landed on disk.
///
/// The GTW-694 screenshot flow only replies once the file `exists()` on disk (or the
/// frame budget elapses): [`Saved`](Self::Saved) carries the constrained
/// [`ScreenshotPathNet`], [`TimedOut`](Self::TimedOut) is the capture-timeout outcome, and
/// [`TargetNotRendered`](Self::TargetNotRendered) is the pre-spawn refusal GTW-922 added.
/// An independent serde enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScreenshotResult {
    /// The screenshot was captured and written to the given path.
    Saved(ScreenshotPathNet),
    /// The capture did not land on disk within the frame budget.
    TimedOut,
    /// No capture was attempted: the offscreen image it would have read is not what the
    /// camera drawing the UI renders into, so the PNG would have been a blank frame
    /// (GTW-922). The carried message names both render targets.
    TargetNotRendered(CaptureAimNet),
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
