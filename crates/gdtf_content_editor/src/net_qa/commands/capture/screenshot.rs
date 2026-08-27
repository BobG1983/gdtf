//! `capture.screenshot` — a PNG of what the editor is showing, attached to the reply.

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
use gdtf_screenshot::{CaptureCompletions, CaptureOutcome, CaptureQueue, CaptureSystems, ShotStem};
use serde::{Deserialize, Serialize};

use crate::net_qa::facts::EditorFacts;

/// Responder type the editor's capture queue carries for this command.
pub(in crate::net_qa) type EditorShotResponder = CommandResponder<EditorCaptureScreenshot>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct CaptureScreenshotArgs {
    #[serde(default)]
    name: Option<ShotName>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct CaptureScreenshotReply {
    path: ArtifactPath,
}

pub(in crate::net_qa) struct EditorCaptureScreenshot;

impl QaCommand for EditorCaptureScreenshot {
    type Args = CaptureScreenshotArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = CaptureScreenshotReply;

    const NAME: CommandName = CommandName::from_static("capture.screenshot");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Write a PNG of what the editor is showing and attach it to the reply. Takes an \
         optional name, which becomes the file stem inside the editor's own shot directory. \
         Needs neither the authoring scene nor a tab, so it answers from the moment the process \
         boots.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(_facts: &EditorFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_capture_screenshot
                .after(QaCommandSystems::Claim)
                .before(CaptureSystems),
        );
    }
}

// Queued before the pump runs, so a request claimed this frame reaches it this frame.
fn handle_editor_capture_screenshot(
    mut calls: ResMut<PendingQueue<CommandCall<EditorCaptureScreenshot>>>,
    mut captures: ResMut<CaptureQueue<EditorShotResponder>>,
    mut completions: ResMut<CaptureCompletions<EditorShotResponder>>,
) {
    for (args, responder) in take_calls::<EditorCaptureScreenshot>(&mut calls) {
        let stem = args.name.map(|name| ShotStem::new(name.as_str()));
        captures.push(stem, responder);
    }
    for completion in completions.drain() {
        let (outcome, responder) = completion.into_parts();
        answer(outcome, responder);
    }
}

fn answer(outcome: CaptureOutcome, responder: EditorShotResponder) {
    match outcome {
        CaptureOutcome::Landed(path) => {
            let artifact = ArtifactPath::new(path.to_string_lossy().into_owned());
            let attachment = ReplyAttachment::new(AttachmentKind::Png, artifact.clone());
            responder.answer_with(&CaptureScreenshotReply { path: artifact }, vec![attachment]);
        }
        CaptureOutcome::TimedOut(path) => {
            debug!(path = %path.display(), "editor net_qa: screenshot capture timed out");
            responder
                .into_inner()
                .reply(QaResponse::Error(QaError::Timeout));
        }
    }
}
