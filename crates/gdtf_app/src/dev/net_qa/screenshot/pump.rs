//! The screenshot capture pump — the GTW-694 architecture's T7 (GTW-740).
//!
//! [`drive_screenshots`] claims each routed
//! [`TakeScreenshot`](gdtf_qa_protocol::envelope::QaRequest::TakeScreenshot) the T3 router
//! queues, hands it a UNIQUE, confined output path under `target/qa_screenshots/`, spawns the
//! REAL render capture ([`Screenshot::primary_window`] + [`save_to_disk`], the
//! `gdtf_screenshot` machinery), and then POLLS the disk across frames — replying
//! [`Saved`](gdtf_qa_protocol::envelope::ScreenshotResult::Saved) ONLY once the PNG verifiably
//! lands ([`inspect_shot`]: exists, non-empty, decodes) or
//! [`TimedOut`](gdtf_qa_protocol::envelope::ScreenshotResult::TimedOut) once its frame budget
//! elapses.
//!
//! ## Why the reply is never a same-frame guess
//!
//! The GPU readback cannot flush the PNG the instant a capture is requested, so a reply that
//! reflects THIS capture must be deferred to a later frame. Three things make that true in
//! code, not just in prose:
//!
//! 1. **Poll BEFORE claim** ([`drive_screenshots`] runs [`poll_in_flight`] then
//!    [`claim_requests`]): a capture claimed this frame is not polled until the NEXT frame, so
//!    the reply is structurally never on the claim frame.
//! 2. **A unique path per capture** ([`next_capture_path`] folds a monotonic
//!    [`ShotSequence`] in): no two captures share a file, so a repeated name never lets one
//!    capture read another's bytes.
//! 3. **Delete-before-spawn** ([`purge_existing`]): any pre-existing file at the path (a
//!    leftover from a prior run at a reused sequence) is removed before the capture spawns, so
//!    the poll can only ever see bytes THIS capture wrote — never a stale PNG.

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};
use gdtf_qa_protocol::envelope::{QaResponse, ScreenshotPathNet, ScreenshotResult};
use gdtf_screenshot::CapturePath;

use super::{
    path::{QaShotDir, ShotSequence, next_capture_path},
    verify::{ShotFile, inspect_shot},
};
use crate::dev::net_qa::{
    channel::Responder,
    pending::{PendingQueue, ScreenshotPayload},
};

crate::support_item! {
    /// The number of frames a capture may poll the disk before the pump gives up and reports
    /// [`TimedOut`](gdtf_qa_protocol::envelope::ScreenshotResult::TimedOut).
    ///
    /// Named-newtype [`Resource`] over `u32` (no-bare-types); the inner is PRIVATE. The
    /// [`Default`] is generous for a real GPU readback (which lands in a handful of frames)
    /// yet bounded to fall inside the listener's reply window (`DEFAULT_IO_TIMEOUT`, 5 s)
    /// at interactive frame rates, so the typed timeout reaches the client rather than the
    /// listener's generic socket timeout. A test pins a tiny budget to drive the timeout
    /// deterministically — the visibility flips to `pub` under `test-support` (the
    /// `test_support` ledger re-exports it so the T7 integration test can inject that budget),
    /// `pub(crate)` otherwise.
    #[derive(Resource, Clone, Copy, Debug, Deref)]
    struct ShotPollBudget(u32);
}

impl ShotPollBudget {
    /// The default poll budget: 240 frames (~4 s at 60 fps).
    const DEFAULT: Self = Self(240);

    crate::support_item! {
        /// Wrap an explicit budget — a test pins a small one (the production plugin uses the
        /// [`Default`]).
        ///
        /// Gated to test / `test-support` builds: the production wiring only ever uses the
        /// [`Default`], so a release build never compiles this constructor.
        #[cfg(any(test, feature = "test-support"))]
        const fn new(frames: u32) -> Self {
            Self(frames)
        }
    }
}

impl Default for ShotPollBudget {
    /// The wiring default: `ShotPollBudget::DEFAULT` (240 frames).
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The frames a single in-flight capture has left to poll before it times out.
///
/// Private-inner countdown newtype over `u32` (no-bare-types); [`tick`](Self::tick) spends
/// one frame and reports whether the budget is now spent.
#[derive(Clone, Copy, Debug)]
struct PollFramesLeft(u32);

/// The outcome of ticking a [`PollFramesLeft`] — a typed alternative to a bare `bool`.
enum PollTick {
    /// Budget remains — keep polling next frame.
    Live,
    /// Budget spent — report the capture timed out.
    Expired,
}

impl PollFramesLeft {
    /// Start a poll countdown of `frames` remaining.
    const fn new(frames: u32) -> Self {
        Self(frames)
    }

    /// Spend one frame of the budget, reporting whether it is now exhausted.
    const fn tick(&mut self) -> PollTick {
        if self.0 == 0 {
            return PollTick::Expired;
        }
        self.0 -= 1;
        PollTick::Live
    }
}

/// One capture spawned and awaiting its PNG: the confined, unique output path, the reply
/// channel, and the poll countdown after which the pump answers a timeout.
struct InFlightShot {
    /// Where the capture is being written (confined + unique under [`QaShotDir`]).
    path:      CapturePath,
    /// The reply channel back to the client.
    responder: Responder,
    /// Frames remaining before the pump reports a timeout.
    remaining: PollFramesLeft,
}

/// The set of captures spawned and polling for their PNG to land — a [`Resource`] the pump
/// carries a request across frames in (the deferred, multi-frame reply state).
#[derive(Resource, Default)]
pub(in crate::dev::net_qa) struct InFlightShots(Vec<InFlightShot>);

/// Poll every in-flight capture, then claim newly-routed requests — replying
/// [`Saved`](ScreenshotResult::Saved) once a capture's PNG verifiably lands or
/// [`TimedOut`](ScreenshotResult::TimedOut) once its budget elapses (GTW-740).
///
/// Registered `.after(route_requests)` by the plugin (so [`claim_requests`] drains the SAME
/// frame's routed pushes) and runs every frame with no battle gate — a screenshot needs no
/// live battle. The POLL runs BEFORE the claim: a capture claimed this frame is only ever
/// polled from the NEXT frame onward, so the reply is structurally never on the claim frame
/// (the readback cannot flush the PNG that instant). Param-only (`bevy-traps.md` #7): the
/// pending queue + in-flight set + sequence counter + the budget / directory config +
/// [`Commands`], no `&mut World`.
pub(in crate::dev::net_qa) fn drive_screenshots(
    mut pending: ResMut<PendingQueue<ScreenshotPayload>>,
    mut in_flight: ResMut<InFlightShots>,
    mut sequence: ResMut<ShotSequence>,
    budget: Res<ShotPollBudget>,
    dir: Res<QaShotDir>,
    mut commands: Commands,
) {
    // Poll first: captures claimed on PRIOR frames may now have landed. A capture claimed
    // below (this frame) is deliberately NOT polled until the next frame.
    poll_in_flight(&mut in_flight);
    claim_requests(
        &mut pending,
        &mut in_flight,
        *budget,
        &dir,
        &mut sequence,
        &mut commands,
    );
}

/// Drain every routed request this frame: hand it a unique confined path, delete any stale
/// file already at that path, ensure the directory exists, spawn the real capture, and start
/// tracking it for the poll (which begins next frame).
fn claim_requests(
    pending: &mut PendingQueue<ScreenshotPayload>,
    in_flight: &mut InFlightShots,
    budget: ShotPollBudget,
    dir: &QaShotDir,
    sequence: &mut ShotSequence,
    commands: &mut Commands,
) {
    for (payload, responder) in pending.drain_ready() {
        let path = next_capture_path(dir, payload.name(), sequence);
        ensure_dir(&path);
        // Delete-before-spawn: a stale PNG at this exact path (a prior run's leftover at a
        // reused sequence) must never be mistaken for THIS capture's output. After the
        // delete, the only file that can appear here is the one this capture writes.
        purge_existing(&path);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk((*path).clone()));
        in_flight.0.push(InFlightShot {
            path,
            responder,
            remaining: PollFramesLeft::new(*budget),
        });
    }
}

/// Poll every in-flight capture: reply [`Saved`](ScreenshotResult::Saved) once its PNG
/// verifiably landed, [`TimedOut`](ScreenshotResult::TimedOut) once its budget elapses, and
/// keep the rest for the next frame.
fn poll_in_flight(in_flight: &mut InFlightShots) {
    if in_flight.0.is_empty() {
        return;
    }
    let mut kept = Vec::with_capacity(in_flight.0.len());
    for mut shot in in_flight.0.drain(..) {
        match inspect_shot(&shot.path) {
            ShotFile::Ready => {
                let saved = ScreenshotPathNet::new(shot.path.to_string_lossy().into_owned());
                shot.responder
                    .reply(QaResponse::Screenshot(ScreenshotResult::Saved(saved)));
            }
            ShotFile::NotReady => match shot.remaining.tick() {
                PollTick::Expired => {
                    debug!(path = %shot.path.display(), "net_qa: screenshot capture timed out");
                    shot.responder
                        .reply(QaResponse::Screenshot(ScreenshotResult::TimedOut));
                }
                PollTick::Live => kept.push(shot),
            },
        }
    }
    in_flight.0 = kept;
}

/// Best-effort: create the confined screenshot directory so the capture can write into it.
/// A failure here surfaces as the poll never finding the PNG (a clean timeout), never a
/// panic (`bevy-traps.md`: no panic in the happy path).
fn ensure_dir(path: &CapturePath) {
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
}

/// Best-effort: remove any pre-existing file at the exact capture path so a stale PNG can
/// never be mistaken for THIS capture's output. A `NotFound` error is the normal case (the
/// path is usually fresh) and is ignored, as is any other error — a residual file that could
/// not be removed simply keeps polling and, absent a fresh landing, times out (never a
/// panic).
fn purge_existing(path: &CapturePath) {
    drop(std::fs::remove_file(&**path));
}
