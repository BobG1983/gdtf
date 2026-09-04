use std::sync::mpsc::Receiver;

use bevy::prelude::*;
use cobalt_mcp_command::dispatch::{CommandCall, CommandResponder};
use cobalt_mcp_protocol::{
    command::{AttachmentKind, CommandOutcome},
    message::{QaError, QaResponse},
};
use cobalt_mcp_transport::{PendingQueue, Responder};
use cobalt_screenshot::{CaptureCompletions, CaptureOutcome, CaptureQueue};

use crate::dev::mcp::commands::capture::screenshot::{
    CaptureScreenshot, ShotResponder, handle_capture_screenshot,
};

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
fn a_timed_out_capture_answers_timeout() {
    let mut app = handler_app();
    let path = cobalt_screenshot::CapturePath::new(std::path::PathBuf::from("target/never.png"));
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
    let path = cobalt_screenshot::CapturePath::new(std::path::PathBuf::from("target/landed_0.png"));
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
