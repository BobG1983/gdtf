//! The screenshot capture pump — the GTW-694 architecture's T7 (GTW-740, retargeted onto
//! the command layer by GTW-943).
//!
//! [`drive_screenshots`] claims each queued capture, hands it a UNIQUE, confined output path
//! under `target/qa_screenshots/`, spawns the REAL render capture ([`Screenshot`] +
//! [`save_to_disk`], the `gdtf_screenshot` machinery) — of the GTW-764 offscreen
//! [`QaCaptureTarget`] image when the present path installed it, else the window swapchain
//! ([`Screenshot::primary_window`]) — and then POLLS the disk across frames, answering with
//! the PNG as a [`ReplyAttachment`] once it verifiably lands ([`inspect_shot`]: exists,
//! non-empty, decodes) or [`Timeout`](QaError::Timeout) once its frame budget elapses.
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
use gdtf_net_qa_transport::{PendingQueue, Responder};
use gdtf_qa_protocol::{
    command::{ArtifactPath, AttachmentKind, CommandOutcome, CommandReplyJson, ReplyAttachment},
    ids::ShotName,
    message::{QaError, QaResponse},
};
use gdtf_screenshot::CapturePath;

use super::{
    path::{QaShotDir, ShotSequence, next_capture_path},
    payload::ScreenshotPayload,
    verify::{ShotFile, inspect_shot},
};
use crate::dev::net_qa::present::QaCaptureTarget;

/// The reply body a landed capture carries.
///
/// The PNG itself rides as a [`ReplyAttachment`], which is how the command layer hands a
/// file back without the courier knowing what a screenshot is. There is no other body to
/// report yet — the command that declares one is the `capture.screenshot` work — so the
/// body is JSON `null` rather than an invented shape.
const CAPTURE_REPLY_BODY: &str = "null";

crate::support_item! {
    /// The number of frames a capture may poll the disk before the pump gives up and reports
    /// [`Timeout`](QaError::Timeout).
    ///
    /// Named-newtype [`Resource`] over `u32` (no-bare-types); the inner is PRIVATE. The
    /// [`Default`] is generous for a real GPU readback (which lands in a handful of frames)
    /// yet bounded to fall inside the listener's reply window (`DEFAULT_IO_TIMEOUT`, 5 s)
    /// at interactive frame rates, so the typed timeout reaches the client rather than the
    /// listener's generic socket timeout. A test pins a tiny budget to drive the timeout
    /// deterministically — the visibility flips to `pub` under `test-support` (the
    /// `test_support` ledger re-exports it so the capture suite can inject that budget),
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

/// Poll every in-flight capture, then claim newly-queued ones — answering with the PNG once
/// a capture verifiably lands, or [`Timeout`](QaError::Timeout) once its budget elapses.
///
/// Registered `.after(route_requests)` by the plugin (so [`claim_requests`] drains the SAME
/// frame's pushes) and runs every frame with no battle gate — a capture needs no live
/// battle. The POLL runs BEFORE the claim: a capture claimed this frame is only ever polled
/// from the NEXT frame onward, so the reply is structurally never on the claim frame (the
/// readback cannot flush the PNG that instant). Param-only (`bevy-traps.md` #7): the
/// pending queue + in-flight set + sequence counter + the budget / directory config + the
/// optional offscreen capture target + [`Commands`], no `&mut World`.
///
/// GTW-764: when the `net_qa` [`QaCaptureTarget`] exists (the env-active present path
/// installed it), the capture reads that OFFSCREEN image (`Screenshot::image`) rather than
/// the window swapchain (`Screenshot::primary_window`) — the swapchain reads back BLACK on a
/// backgrounded macOS window. `Option`, so a build without the present path falls back to the
/// window.
pub(in crate::dev::net_qa) fn drive_screenshots(
    mut pending: ResMut<PendingQueue<ScreenshotPayload>>,
    mut in_flight: ResMut<InFlightShots>,
    mut sequence: ResMut<ShotSequence>,
    budget: Res<ShotPollBudget>,
    dir: Res<QaShotDir>,
    capture_target: Option<Res<QaCaptureTarget>>,
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
        capture_target.as_deref(),
        &mut commands,
    );
}

/// Drain every queued capture this frame: hand each a unique confined path and start
/// tracking it for the poll (which begins next frame) via [`spawn_capture`].
fn claim_requests(
    pending: &mut PendingQueue<ScreenshotPayload>,
    in_flight: &mut InFlightShots,
    budget: ShotPollBudget,
    dir: &QaShotDir,
    sequence: &mut ShotSequence,
    target: Option<&QaCaptureTarget>,
    commands: &mut Commands,
) {
    for (payload, responder) in pending.drain_ready() {
        spawn_capture(
            payload.name(),
            responder,
            CaptureSink {
                in_flight: &mut *in_flight,
                budget,
                dir,
                sequence: &mut *sequence,
                target,
                commands: &mut *commands,
            },
        );
    }
}

/// The capture-launch bundle every confined capture spawns through: the in-flight
/// tracking set, the poll budget, the confinement directory, the path-uniqueness
/// sequence, and [`Commands`]. Bundled into ONE param so [`spawn_capture`] stays under
/// the clippy argument-count ceiling.
struct CaptureSink<'a, 'w, 's> {
    /// The captures in-flight across frames, this one joins.
    in_flight: &'a mut InFlightShots,
    /// The frames a capture may poll the disk before timing out.
    budget:    ShotPollBudget,
    /// The directory every capture is confined under.
    dir:       &'a QaShotDir,
    /// The monotonic per-capture uniqueness counter.
    sequence:  &'a mut ShotSequence,
    /// The offscreen capture target to read (GTW-764), or `None` to fall back to the window
    /// swapchain. Present on the env-active `net_qa` path; absent otherwise.
    target:    Option<&'a QaCaptureTarget>,
    /// The command queue the real capture entity spawns through.
    commands:  &'a mut Commands<'w, 's>,
}

/// Hand `responder` a unique confined path, delete any stale file already at that path,
/// ensure the directory exists, spawn the real capture, and start tracking it in
/// `sink`'s in-flight set for the poll (which begins next frame).
fn spawn_capture(name: Option<&ShotName>, responder: Responder, sink: CaptureSink<'_, '_, '_>) {
    let path = next_capture_path(sink.dir, name, sink.sequence);
    ensure_dir(&path);
    // Delete-before-spawn: a stale PNG at this exact path (a prior run's leftover at a
    // reused sequence) must never be mistaken for THIS capture's output. After the
    // delete, the only file that can appear here is the one this capture writes.
    purge_existing(&path);
    // GTW-764: capture the OFFSCREEN image the render graph writes every tick when the
    // present path installed it — a backgrounded macOS window's swapchain reads back BLACK.
    // With no target (a build without the present path), fall back to the window swapchain.
    let screenshot = match sink.target {
        Some(target) => Screenshot::image((**target).clone()),
        None => Screenshot::primary_window(),
    };
    sink.commands
        .spawn(screenshot)
        .observe(save_to_disk((*path).clone()));
    sink.in_flight.0.push(InFlightShot {
        path,
        responder,
        remaining: PollFramesLeft::new(*sink.budget),
    });
}

/// The reply for a capture that verifiably landed at `path`.
///
/// The file rides as a [`ReplyAttachment`], which the courier resolves against the CHILD's
/// working directory and emits as its own content block — one generic rule, no per-command
/// arm.
fn landed_reply(path: &CapturePath) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::Ran {
        reply:       CommandReplyJson::new(CAPTURE_REPLY_BODY.to_owned()),
        attachments: vec![ReplyAttachment::new(
            AttachmentKind::Png,
            ArtifactPath::new(path.to_string_lossy().into_owned()),
        )],
    })
}

/// Poll every in-flight capture: answer with the PNG once it verifiably landed,
/// [`Timeout`](QaError::Timeout) once its budget elapses, and keep the rest for the next
/// frame.
fn poll_in_flight(in_flight: &mut InFlightShots) {
    if in_flight.0.is_empty() {
        return;
    }
    let mut kept = Vec::with_capacity(in_flight.0.len());
    for mut shot in in_flight.0.drain(..) {
        match inspect_shot(&shot.path) {
            ShotFile::Ready => {
                let response = landed_reply(&shot.path);
                shot.responder.reply(response);
            }
            ShotFile::NotReady => match shot.remaining.tick() {
                PollTick::Expired => {
                    debug!(path = %shot.path.display(), "net_qa: screenshot capture timed out");
                    shot.responder.reply(QaResponse::Error(QaError::Timeout));
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
