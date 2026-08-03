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

#[derive(Clone, Copy, Debug)]
struct FramesLeft(u32);

enum FrameTick {
        Live,
        Expired,
}

impl FramesLeft {
        const fn new(frames: u32) -> Self {
        Self(frames)
    }

        const fn tick(&mut self) -> FrameTick {
        if self.0 == 0 {
            return FrameTick::Expired;
        }
        self.0 -= 1;
        FrameTick::Live
    }
}

enum ShotStage {
        Settling(FramesLeft),
        Capturing(FramesLeft),
}

struct EditorShot {
        path:      CapturePath,
        responder: Responder,
        stage:     ShotStage,
}

#[derive(Resource, Default)]
pub(in crate::net_qa) struct EditorInFlightShots(Vec<EditorShot>);

pub(in crate::net_qa) fn drive_editor_screenshots(
    mut pending: ResMut<PendingQueue<EditorScreenshotPayload>>,
    mut in_flight: ResMut<EditorInFlightShots>,
    mut sequence: ResMut<EditorShotSequence>,
    tunables: ShotTunables,
    aim: EditorCaptureAim,
    mut commands: Commands,
) {
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

#[derive(SystemParam)]
pub(in crate::net_qa) struct ShotTunables<'w> {
        settle: Res<'w, EditorShotSettle>,
        budget: Res<'w, EditorShotPollBudget>,
        dir:    Res<'w, EditorQaShotDir>,
        source: Res<'w, EditorShotSource>,
}

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

const CAPTURE_REPLY_BODY: &str = "null";

fn landed_reply(path: &CapturePath) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::Ran {
        reply:       CommandReplyJson::new(CAPTURE_REPLY_BODY.to_owned()),
        attachments: vec![ReplyAttachment::new(
            AttachmentKind::Png,
            ArtifactPath::new(path.to_string_lossy().into_owned()),
        )],
    })
}

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
