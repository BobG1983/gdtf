use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, CommandResponder, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::{
    command::{
        ArtifactPath, AttachmentKind, CommandAvailability, CommandName, CommandSummary,
        CommandTiming, ReplyAttachment,
    },
    ids::ShotName,
    message::{QaError, QaResponse},
};
use gdtf_screenshot::{CaptureCompletions, CaptureOutcome, CaptureQueue, ShotStem};
use serde::{Deserialize, Serialize};

use crate::dev::net_qa::facts::GameFacts;

/// Responder type the capture queue carries for this command.
pub(in crate::dev::net_qa) type ShotResponder = CommandResponder<CaptureScreenshot>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CaptureScreenshotArgs {
    #[serde(default)]
    name: Option<ShotName>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CaptureScreenshotReply {
    path: ArtifactPath,
}

pub(crate) struct CaptureScreenshot;

impl QaCommand for CaptureScreenshot {
    type Args = CaptureScreenshotArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = CaptureScreenshotReply;

    const NAME: CommandName = CommandName::from_static("capture.screenshot");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Write a PNG of what the game is showing and attach it to the reply. Takes an \
         optional name, which becomes the file stem inside the host's shot directory. \
         Needs no battle, so it answers from the moment the process boots.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_capture_screenshot.after(QaCommandSystems::Claim),
        );
    }
}

pub(in crate::dev::net_qa) fn handle_capture_screenshot(
    mut calls: ResMut<PendingQueue<CommandCall<CaptureScreenshot>>>,
    mut captures: ResMut<CaptureQueue<ShotResponder>>,
    mut completions: ResMut<CaptureCompletions<ShotResponder>>,
) {
    for (args, responder) in take_calls::<CaptureScreenshot>(&mut calls) {
        let stem = args.name.map(|name| ShotStem::new(name.as_str()));
        captures.push(stem, responder);
    }
    for completion in completions.drain() {
        let (outcome, responder) = completion.into_parts();
        answer(outcome, responder);
    }
}

fn answer(outcome: CaptureOutcome, responder: ShotResponder) {
    match outcome {
        CaptureOutcome::Landed(path) => {
            let artifact = ArtifactPath::new(path.to_string_lossy().into_owned());
            let attachment = ReplyAttachment::new(AttachmentKind::Png, artifact.clone());
            responder.answer_with(&CaptureScreenshotReply { path: artifact }, vec![attachment]);
        }
        CaptureOutcome::TimedOut(path) => {
            debug!(path = %path.display(), "net_qa: screenshot capture timed out");
            responder
                .into_inner()
                .reply(QaResponse::Error(QaError::Timeout));
        }
    }
}
