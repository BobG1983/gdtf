use std::sync::mpsc::Receiver;

use bevy::prelude::*;
use gdtf_net_qa_transport::{PendingQueue, Responder};
use gdtf_qa_command::dispatch::{CommandCall, CommandResponder};
use gdtf_qa_protocol::{
    command::{AttachmentKind, CommandOutcome, UnavailableCode},
    message::{QaError, QaResponse},
};
use gdtf_screenshot::{CaptureAimDetail, CaptureCompletions, CaptureOutcome, CaptureQueue};

use crate::dev::net_qa::commands::capture::screenshot::{
    CaptureScreenshot, ShotResponder, handle_capture_screenshot,
};

const REFUSAL_DETAIL: &str = "nothing renders into the image this capture would read";

fn handler_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<PendingQueue<CommandCall<CaptureScreenshot>>>();
    app.init_resource::<CaptureQueue<ShotResponder>>();
    app.init_resource::<CaptureCompletions<ShotResponder>>();
    app.add_systems(Update, handle_capture_screenshot);
    app
}

fn complete_with(app: &mut App, outcome: CaptureOutcome) -> Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    app.world_mut()
        .resource_mut::<CaptureCompletions<ShotResponder>>()
        .push(outcome, CommandResponder::new(responder));
    reply_rx
}

#[test]
fn a_refused_capture_answers_unavailable_with_wrong_state_and_the_detail() {
    let mut app = handler_app();
    let detail = CaptureAimDetail::new(REFUSAL_DETAIL.to_owned());
    let reply_rx = complete_with(&mut app, CaptureOutcome::Refused(detail));

    app.update();

    let reply = reply_rx.try_recv().ok();
    let Some(QaResponse::Outcome(CommandOutcome::Unavailable { code, note })) = &reply else {
        unreachable!("a refused capture must answer Unavailable, got {reply:?}");
    };
    assert_eq!(
        *code,
        UnavailableCode::WrongState,
        "a capture the host refuses because nothing renders into its target is a WRONG STATE \
         refusal, not NotBuilt and not Unsupported; got {reply:?}",
    );
    assert!(
        note.as_str().contains(REFUSAL_DETAIL),
        "the refusal must carry the aim check's own detail so a caller can see the mismatch; it \
         said {:?}",
        note.as_str(),
    );
}

#[test]
fn a_timed_out_capture_answers_timeout() {
    let mut app = handler_app();
    let path = gdtf_screenshot::CapturePath::new(std::path::PathBuf::from("target/never.png"));
    let reply_rx = complete_with(&mut app, CaptureOutcome::TimedOut(path));

    app.update();

    let reply = reply_rx.try_recv().ok();
    assert!(
        matches!(reply, Some(QaResponse::Error(QaError::Timeout))),
        "a capture whose PNG never lands must answer Timeout, never an attachment; got {reply:?}",
    );
}

#[test]
fn a_landed_capture_attaches_the_png_it_wrote() {
    let mut app = handler_app();
    let path = gdtf_screenshot::CapturePath::new(std::path::PathBuf::from("target/landed_0.png"));
    let reply_rx = complete_with(&mut app, CaptureOutcome::Landed(path.clone()));

    app.update();

    let reply = reply_rx.try_recv().ok();
    let Some(QaResponse::Outcome(CommandOutcome::Ran { attachments, .. })) = &reply else {
        unreachable!("a landed capture must answer Ran, got {reply:?}");
    };

    let attached = attachments
        .first()
        .filter(|attachment| attachment.kind == AttachmentKind::Png)
        .map(|attachment| attachment.path.as_str().to_owned());
    assert_eq!(
        attached,
        Some(path.to_string_lossy().into_owned()),
        "a landed capture must come back as a PNG attachment at that exact path, got {reply:?}",
    );
}
