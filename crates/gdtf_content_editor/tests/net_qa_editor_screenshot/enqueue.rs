//! Enqueue one capture through the editor's REAL pending queue (GTW-943).
//!
//! This used to be the socket half: a framed `TakeScreenshot` over a real `TcpStream`. That
//! request went with the rest of the pre-command vocabulary, and the editor's own capture
//! command is the next editor ticket, so there is no wire route into the pump right now.
//!
//! What is left is still the production path on the host side — the same
//! [`PendingQueue`](gdtf_net_qa_transport::PendingQueue) the router pushed onto, the same
//! [`Responder`] the listener thread hands over, and the same pump claiming it. Only the
//! socket in front of it is gone.

use std::sync::mpsc::Receiver;

use bevy::prelude::*;
use gdtf_content_editor::EditorScreenshotPayload;
use gdtf_net_qa_transport::{PendingQueue, Responder};
use gdtf_qa_protocol::{ids::ShotName, message::QaResponse};

/// Push one capture onto the editor's real queue and hand back the channel its reply
/// arrives on.
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
