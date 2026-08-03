use std::sync::mpsc::Receiver;

use bevy::prelude::*;
use gdtf_content_editor::EditorScreenshotPayload;
use gdtf_net_qa_transport::{PendingQueue, Responder};
use gdtf_qa_protocol::{ids::ShotName, message::QaResponse};

pub(crate) fn enqueue_capture(app: &mut App, name: &str) -> Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    app.world_mut()
        .resource_mut::<PendingQueue<EditorScreenshotPayload>>()
        .push_new(
            EditorScreenshotPayload::new(Some(ShotName::new(name.to_owned()))),
            responder,
        );
    reply_rx
}
