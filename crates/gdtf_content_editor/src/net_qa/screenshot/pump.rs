//! The editor's screenshot capture pump (GTW-880).
//!
//! [`drive_editor_screenshots`] claims each queued capture, hands it a UNIQUE, confined
//! output path under [`EditorQaShotDir`], lets the
//! shell settle, spawns the REAL render capture ([`spawn_capture`] — a `Screenshot` plus
//! `save_to_disk`, the `gdtf_screenshot` machinery), and then POLLS the disk across frames — replying
//! with the PNG as an attachment ONLY once it verifiably lands ([`inspect_shot`]:
//! exists, non-empty, decodes) or [`Timeout`](QaError::Timeout) once its poll
//! budget elapses.
//!
//! ## Why the reply can never come before the PNG
//!
//! The GPU readback cannot flush the PNG the instant a capture is requested, so a reply that
//! reflects THIS capture must be deferred across frames. Four things make that true in code,
//! not just in prose:
//!
//! 1. **Advance BEFORE claim.** [`drive_editor_screenshots`] runs [`advance_in_flight`] and
//!    THEN [`claim_requests`], so a capture claimed this frame is not advanced until the NEXT
//!    frame — the reply is structurally never on the claim frame.
//! 2. **Settle before the capture is even spawned.** A claimed capture spends
//!    [`EditorShotSettle`] frames in [`ShotStage::Settling`] before the capture is spawned, so
//!    the PNG shows a laid-out egui shell rather than a mid-layout frame.
//! 3. **The reply is driven by the FILE.** [`ShotStage::Capturing`] replies only on
//!    [`ShotFile::Ready`] — the bytes on disk read back and decoded as a PNG. No timer, no
//!    optimistic "it should have landed by now".
//! 4. **Delete-before-spawn.** [`spawn_capture`] removes any pre-existing file at the exact
//!    path before the capture spawns, so the poll can only ever see bytes THIS capture wrote.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_net_qa_transport::{PendingQueue, Responder};
use gdtf_qa_protocol::{
    command::{
        ArtifactPath, AttachmentKind, CommandOutcome, CommandReplyJson, RefusalNote,
        ReplyAttachment, UnavailableCode,
    },
    ids::ShotName,
    message::{QaError, QaResponse},
};
use gdtf_screenshot::CapturePath;

use super::{
    aim::{CaptureAim, EditorCaptureAim},
    config::{EditorShotPollBudget, EditorShotSettle, EditorShotSource},
    path::{EditorQaShotDir, EditorShotSequence, next_capture_path},
    payload::EditorScreenshotPayload,
    spawn::{ensure_dir, spawn_capture},
    verify::{ShotFile, inspect_shot},
};

/// A frames-remaining countdown, used for both the settle window and the disk poll.
///
/// Private-inner newtype over `u32` (no-bare-types); [`tick`](Self::tick) spends one frame and
/// reports whether the budget is now spent.
#[derive(Clone, Copy, Debug)]
struct FramesLeft(u32);

/// The outcome of ticking a [`FramesLeft`] — a typed alternative to a bare `bool`.
enum FrameTick {
    /// Budget remains — carry on next frame.
    Live,
    /// Budget spent — take this stage's expiry action.
    Expired,
}

impl FramesLeft {
    /// Start a countdown of `frames` remaining.
    const fn new(frames: u32) -> Self {
        Self(frames)
    }

    /// Spend one frame of the budget, reporting whether it is now exhausted.
    const fn tick(&mut self) -> FrameTick {
        if self.0 == 0 {
            return FrameTick::Expired;
        }
        self.0 -= 1;
        FrameTick::Live
    }
}

/// Where one in-flight capture is in its life: settling, then polling for its PNG.
enum ShotStage {
    /// Waiting out the settle window before the capture is spawned.
    Settling(FramesLeft),
    /// The capture is spawned; polling the disk for its PNG.
    Capturing(FramesLeft),
}

/// One claimed capture the pump carries across frames: its confined, unique output path, the
/// reply channel, and where it is in its life.
struct EditorShot {
    /// Where the capture will be (or is being) written, confined under [`EditorQaShotDir`].
    path:      CapturePath,
    /// The reply channel back to the client.
    responder: Responder,
    /// Settling, or spawned and polling.
    stage:     ShotStage,
}

/// The captures claimed and not yet answered — a [`Resource`] the pump carries a request
/// across frames in (the deferred, multi-frame reply state).
#[derive(Resource, Default)]
pub(in crate::net_qa) struct EditorInFlightShots(Vec<EditorShot>);

/// Advance every in-flight capture, then claim newly-routed requests (see the module doc for
/// why that order is the ordering contract).
///
/// Registered `.after(route_editor_requests)` by the plugin, so [`claim_requests`] drains the
/// SAME frame's routed pushes and the entry never reaches the transport's deadline sweep.
/// Runs in every editor state — a screenshot of the `Load` screen is as legitimate as one of
/// the `Editing` shell.
///
/// Param-only (`bevy-traps.md` #7): the pending queue + the in-flight set + the sequence
/// counter + the three config resources + [`Commands`], no `&mut World`.
pub(in crate::net_qa) fn drive_editor_screenshots(
    mut pending: ResMut<PendingQueue<EditorScreenshotPayload>>,
    mut in_flight: ResMut<EditorInFlightShots>,
    mut sequence: ResMut<EditorShotSequence>,
    tunables: ShotTunables,
    aim: EditorCaptureAim,
    mut commands: Commands,
) {
    // Advance first: captures claimed on PRIOR frames may now be due to spawn, or may have
    // landed. A capture claimed below (this frame) is deliberately NOT advanced until the
    // next frame.
    advance_in_flight(
        &mut in_flight,
        *tunables.budget,
        &tunables.source,
        &aim,
        &mut commands,
    );
    claim_requests(
        &mut pending,
        &mut in_flight,
        *tunables.settle,
        &tunables.dir,
        &mut sequence,
    );
}

/// The pump's four configuration reads, bundled into ONE [`SystemParam`] so
/// [`drive_editor_screenshots`] stays under the clippy argument-count ceiling — the same
/// bundling the game's router does for its pending queues.
#[derive(SystemParam)]
pub(in crate::net_qa) struct ShotTunables<'w> {
    /// How long a claimed capture settles before it is spawned.
    settle: Res<'w, EditorShotSettle>,
    /// How long a spawned capture polls the disk before it times out.
    budget: Res<'w, EditorShotPollBudget>,
    /// The directory every capture is confined under.
    dir:    Res<'w, EditorQaShotDir>,
    /// Which pixels a capture reads.
    source: Res<'w, EditorShotSource>,
}

/// Drain every routed request this frame: hand each a unique confined path, make sure its
/// directory exists, and start its settle countdown.
fn claim_requests(
    pending: &mut PendingQueue<EditorScreenshotPayload>,
    in_flight: &mut EditorInFlightShots,
    settle: EditorShotSettle,
    dir: &EditorQaShotDir,
    sequence: &mut EditorShotSequence,
) {
    for (payload, responder) in pending.drain_ready() {
        in_flight
            .0
            .push(claim_one(payload.name(), responder, settle, dir, sequence));
    }
}

/// Start tracking ONE claimed capture: its unique confined path, its directory, and its
/// settle countdown. Nothing is spawned here — that happens when the settle window elapses, on a
/// later frame.
fn claim_one(
    name: Option<&ShotName>,
    responder: Responder,
    settle: EditorShotSettle,
    dir: &EditorQaShotDir,
    sequence: &mut EditorShotSequence,
) -> EditorShot {
    let path = next_capture_path(dir, name, sequence);
    ensure_dir(&path);
    EditorShot {
        path,
        responder,
        stage: ShotStage::Settling(FramesLeft::new(**settle)),
    }
}

/// The reply body a landed capture carries.
///
/// The PNG itself rides as a [`ReplyAttachment`], which is how the command layer hands a file
/// back without the courier knowing what a screenshot is. There is no other body to report
/// yet — the editor command that declares one is the editor-host ticket — so the body is JSON
/// `null` rather than an invented shape.
const CAPTURE_REPLY_BODY: &str = "null";

/// The reply for a capture that verifiably landed at `path`.
fn landed_reply(path: &CapturePath) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::Ran {
        reply:       CommandReplyJson::new(CAPTURE_REPLY_BODY.to_owned()),
        attachments: vec![ReplyAttachment::new(
            AttachmentKind::Png,
            ArtifactPath::new(path.to_string_lossy().into_owned()),
        )],
    })
}

/// Advance every in-flight capture one frame: spawn a settled capture, hand back the PNG as
/// an attachment once a spawned capture verifiably landed, answer
/// [`Timeout`](QaError::Timeout) once its poll budget elapses, and keep the rest for the next
/// frame.
fn advance_in_flight(
    in_flight: &mut EditorInFlightShots,
    budget: EditorShotPollBudget,
    source: &EditorShotSource,
    aim: &EditorCaptureAim,
    commands: &mut Commands,
) {
    if in_flight.0.is_empty() {
        return;
    }
    let mut kept = Vec::with_capacity(in_flight.0.len());
    for mut shot in in_flight.0.drain(..) {
        match &mut shot.stage {
            ShotStage::Settling(remaining) => {
                if let FrameTick::Expired = remaining.tick() {
                    // The pre-spawn consistency check (GTW-922): a capture that would read an
                    // offscreen target the editor's UI camera is not drawing into is REFUSED,
                    // with the mismatch named, rather than answered with a blank PNG.
                    if let CaptureAim::Refused(detail) = aim.verify(source) {
                        warn!(
                            detail = %detail.as_str(),
                            "editor net_qa: refusing a screenshot of an offscreen target nothing \
                             renders into",
                        );
                        shot.responder
                            .reply(QaResponse::Outcome(CommandOutcome::Unavailable {
                                code: UnavailableCode::WrongState,
                                note: RefusalNote::from_owned(detail.as_str().to_owned()),
                            }));
                        continue;
                    }
                    spawn_capture(&shot.path, source, commands);
                    shot.stage = ShotStage::Capturing(FramesLeft::new(**budget));
                }
                kept.push(shot);
            }
            ShotStage::Capturing(remaining) => match inspect_shot(&shot.path) {
                ShotFile::Ready => {
                    shot.responder.reply(landed_reply(&shot.path));
                }
                ShotFile::NotReady => match remaining.tick() {
                    FrameTick::Expired => {
                        debug!(
                            path = %shot.path.display(),
                            "editor net_qa: screenshot capture timed out",
                        );
                        shot.responder.reply(QaResponse::Error(QaError::Timeout));
                    }
                    FrameTick::Live => kept.push(shot),
                },
            },
        }
    }
    in_flight.0 = kept;
}
