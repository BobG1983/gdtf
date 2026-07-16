//! [`ScreenshotResult`] + [`ScreenshotPathNet`] — the reply to a screenshot request
//! (GTW-734).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

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
