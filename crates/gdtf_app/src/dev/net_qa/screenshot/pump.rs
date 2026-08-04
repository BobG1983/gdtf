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

const CAPTURE_REPLY_BODY: &str = "null";

crate::support_item! {
                                                #[derive(Resource, Clone, Copy, Debug, Deref)]
    struct ShotPollBudget(u32);
}

impl ShotPollBudget {
    const DEFAULT: Self = Self(240);

    crate::support_item! {
                                                #[cfg(any(test, feature = "test-support"))]
        const fn new(frames: u32) -> Self {
            Self(frames)
        }
    }
}

impl Default for ShotPollBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Copy, Debug)]
struct PollFramesLeft(u32);

enum PollTick {
    Live,
    Expired,
}

impl PollFramesLeft {
    const fn new(frames: u32) -> Self {
        Self(frames)
    }

    const fn tick(&mut self) -> PollTick {
        if self.0 == 0 {
            return PollTick::Expired;
        }
        self.0 -= 1;
        PollTick::Live
    }
}

struct InFlightShot {
    path:      CapturePath,
    responder: Responder,
    remaining: PollFramesLeft,
}

#[derive(Resource, Default)]
pub(in crate::dev::net_qa) struct InFlightShots(Vec<InFlightShot>);

pub(in crate::dev::net_qa) fn drive_screenshots(
    mut pending: ResMut<PendingQueue<ScreenshotPayload>>,
    mut in_flight: ResMut<InFlightShots>,
    mut sequence: ResMut<ShotSequence>,
    budget: Res<ShotPollBudget>,
    dir: Res<QaShotDir>,
    capture_target: Option<Res<QaCaptureTarget>>,
    mut commands: Commands,
) {
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

struct CaptureSink<'a, 'w, 's> {
    in_flight: &'a mut InFlightShots,
    budget:    ShotPollBudget,
    dir:       &'a QaShotDir,
    sequence:  &'a mut ShotSequence,
    target:    Option<&'a QaCaptureTarget>,
    commands:  &'a mut Commands<'w, 's>,
}

fn spawn_capture(name: Option<&ShotName>, responder: Responder, sink: CaptureSink<'_, '_, '_>) {
    let path = next_capture_path(sink.dir, name, sink.sequence);
    ensure_dir(&path);
    purge_existing(&path);
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

fn landed_reply(path: &CapturePath) -> QaResponse {
    QaResponse::Outcome(CommandOutcome::Ran {
        reply:       CommandReplyJson::new(CAPTURE_REPLY_BODY.to_owned()),
        attachments: vec![ReplyAttachment::new(
            AttachmentKind::Png,
            ArtifactPath::new(path.to_string_lossy().into_owned()),
        )],
    })
}

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

fn ensure_dir(path: &CapturePath) {
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
}

fn purge_existing(path: &CapturePath) {
    drop(std::fs::remove_file(&**path));
}
