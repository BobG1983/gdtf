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
/// [`Offscreen`](Self::Offscreen) is the DEFAULT (GTW-918) and is what the running editor
/// captures: an image the editor's egui camera renders into every tick, so a capture is
/// independent of window presentation. [`PrimaryWindow`](Self::PrimaryWindow) reads the window
/// swapchain and is now only reachable when a caller inserts it.
///
/// Which arm is installed is pinned by
/// `test/source.rs::the_plugin_installs_the_shipped_capture_source`, so this [`Default`]
/// cannot change without a test changing with it.
#[derive(Resource, Clone, Debug)]
pub enum EditorShotSource {
    /// The primary window's swapchain — the editor's own on-screen window.
    ///
    /// NOT what the running editor captures any more (GTW-918). Its remaining role is the one
    /// the game keeps for the same arm: the FALLBACK for a build that has no present path. Two
    /// ways it is reached, both deliberate — a caller (a test, or an editor configuration wired
    /// without `EditorCapturePresentPlugin`) inserting it by name, and the pump's own fallback
    /// when the [`Offscreen`](Self::Offscreen) arm still carries the [`Default`]'s placeholder
    /// handle, i.e. before any offscreen target has been created (see `pump.rs`'s
    /// `spawn_capture` match and the [`Default`] impl below).
    ///
    /// Why it is not the default: it works only while the window is VISIBLE. Per GTW-764, on
    /// macOS a backgrounded, occluded or minimized window's Metal drawable is stale and the
    /// copy reads back black — and an unattended agent-QA run is exactly that condition.
    ///
    /// No test observes a real window readback through this arm (a test app has no window to
    /// read back), so the coverage that exists is the target-selection assertion in
    /// `test/source.rs`, which pins this arm to
    /// `Screenshot(RenderTarget::Window(WindowRef::Primary))`.
    PrimaryWindow,
    /// An offscreen image the render graph writes every tick — the editor's shipped source.
    ///
    /// The running editor's handle is created and installed by the present path
    /// (`crate::net_qa::present`), which sizes the image to the window, adds `COPY_SRC`, and
    /// retargets the egui camera into it.
    Offscreen(Handle<Image>),
}

impl Default for EditorShotSource {
    /// The wiring default: [`Offscreen`](Self::Offscreen), so the swapchain is unreachable on
    /// the listener arm unless a caller deliberately asks for it (GTW-918 clause 2).
    ///
    /// The handle is a PLACEHOLDER — `#[derive(Default)]` cannot pick a variant that carries
    /// data, and the plugin installs this value through `init_resource` before any window
    /// exists to size a real target against. `ensure_editor_capture_target` replaces it with the
    /// handle of the image the egui camera actually renders into, on the first frame the editor
    /// has a sized primary window.
    ///
    /// What that placeholder handle actually names, stated exactly: `Handle::<Image>::default()`
    /// is `Handle::Uuid(AssetId::DEFAULT_UUID)`, and `ImagePlugin::build` REGISTERS Bevy's 1x1
    /// white `Image::default()` at precisely that handle
    /// (`bevy_image-0.19.0/src/image.rs:220-222`). So it does not name "no image" — capturing
    /// through it would land a 1x1 white PNG as if it were the editor's shell, or fail the copy
    /// because that image's descriptor carries no `COPY_SRC`. Neither is acceptable in the path
    /// agent QA drives, so `pump.rs`'s `spawn_capture` matches the placeholder handle and takes
    /// the window swapchain instead — the same fallback the game's pump takes when no capture
    /// target resource exists (`crates/gdtf_app/src/dev/net_qa/screenshot/pump.rs:262-266`).
    /// `test/source.rs::the_default_placeholder_source_falls_back_to_the_primary_window`
    /// pins that.
    fn default() -> Self {
        Self::Offscreen(Handle::default())
    }
}
