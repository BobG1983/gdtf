//! The capture pump's tunables: the settle window, the poll budget, and which pixels a
//! capture reads (GTW-880).
//!
//! All three are [`Resource`]s with a documented [`Default`] the plugin wires, and all three
//! are `pub` (re-exported from the crate root under the same `net_qa` gate) so the
//! integration suite can pin a small settle / budget and point a capture at an offscreen
//! image. None of them is shared with the older `GDTF_EDITOR_SHOT` affordance: that path
//! carries its own [`SettleFrames`] / [`PollCap`] resources, and inserting those types here
//! would make the two capture paths fight over one value. The calibrated NUMBERS are still
//! the shared crate's — this module wraps them rather than restating them.

use bevy::{image::Image, prelude::*};
use gdtf_screenshot::{PollCap, SettleFrames};

/// How many frames a claimed capture waits before the real capture is spawned.
///
/// Named-newtype [`Resource`] over the shared [`SettleFrames`] (no-bare-types); the inner is
/// PRIVATE. A screenshot taken mid-layout shows a half-drawn shell, so the editor settles
/// first — the [`Default`] is [`SettleFrames::DEFAULT_EGUI`], the value the editor's own
/// `GDTF_EDITOR_SHOT` capture is calibrated to for exactly this reason.
#[derive(Resource, Clone, Copy, Debug, Deref)]
pub struct EditorShotSettle(SettleFrames);

impl EditorShotSettle {
    /// Wrap an explicit settle window — the integration suite pins a short one (the plugin's
    /// own wiring uses the [`Default`]).
    #[must_use]
    pub const fn new(frames: SettleFrames) -> Self {
        Self(frames)
    }
}

impl Default for EditorShotSettle {
    /// The wiring default: the shared crate's egui-calibrated settle window.
    fn default() -> Self {
        Self(SettleFrames::DEFAULT_EGUI)
    }
}

/// How many frames a spawned capture may poll the disk before the pump gives up and reports
/// [`TimedOut`](gdtf_qa_protocol::envelope::ScreenshotResult::TimedOut).
///
/// Named-newtype [`Resource`] over the shared [`PollCap`] (no-bare-types); the inner is
/// PRIVATE. The [`Default`] is [`PollCap::DEFAULT`] — generous for a real GPU readback, and
/// bounded so a failed write reports a typed timeout instead of hanging the client until its
/// socket read timeout reaps it.
#[derive(Resource, Clone, Copy, Debug, Deref)]
pub struct EditorShotPollBudget(PollCap);

impl EditorShotPollBudget {
    /// Wrap an explicit poll budget — the integration suite pins a small one (the plugin's
    /// own wiring uses the [`Default`]).
    #[must_use]
    pub const fn new(frames: PollCap) -> Self {
        Self(frames)
    }
}

impl Default for EditorShotPollBudget {
    /// The wiring default: the shared crate's calibrated poll cap.
    fn default() -> Self {
        Self(PollCap::DEFAULT)
    }
}

/// Which pixels an editor QA capture reads.
///
/// A named two-case [`Resource`] rather than an `Option<Handle<Image>>` (no-bare-types): the
/// two cases carry the domain meaning "the editor's real screen" and "an offscreen image the
/// render graph writes every tick", and the [`Default`] states which one the editor binary
/// uses.
///
/// [`PrimaryWindow`](Self::PrimaryWindow) is the default and is what the running editor
/// captures — the same source the editor's `GDTF_EDITOR_SHOT` affordance has always read.
/// [`Offscreen`](Self::Offscreen) is the editor's hook for the game's GTW-764 answer to the
/// macOS problem that a backgrounded window's swapchain reads back black: point it at an
/// image a camera renders into and the capture is independent of window presentation. The
/// integration suite uses it because a headless test app has no window at all.
///
/// Which arm is installed is pinned by
/// `test/source.rs::the_plugin_installs_the_shipped_capture_source`, so this `#[default]`
/// cannot change without a test changing with it.
#[derive(Resource, Clone, Debug, Default)]
pub enum EditorShotSource {
    /// The primary window's swapchain — the running editor's own screen.
    ///
    /// GTW-917: no test observes a real window readback through this arm — a test app has no
    /// window to read back — so the coverage that exists is the target-selection assertion in
    /// `test/source.rs`, which pins this arm to
    /// `Screenshot(RenderTarget::Window(WindowRef::Primary))`. Per GTW-764 this source reads
    /// back black from a backgrounded, occluded or minimized macOS window; GTW-918 is why the
    /// running editor stops taking it, by giving the editor an offscreen render target and
    /// making [`Offscreen`](Self::Offscreen) the source it captures through.
    #[default]
    PrimaryWindow,
    /// An offscreen image the render graph writes every tick.
    Offscreen(Handle<Image>),
}
