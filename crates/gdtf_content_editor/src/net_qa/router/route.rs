//! [`route_editor_requests`] — the ONE drain of the editor's [`NetInbox`] (GTW-804,
//! GTW-943).
//!
//! It answers the two command-layer requests. The editor has no command set yet — building
//! one is the editor-host ticket — so its catalogue is EMPTY and every `Run` comes back
//! [`Unknown`](gdtf_qa_protocol::command::CommandOutcome::Unknown) listing the (empty) set of
//! names it does know. That is an honest answer a client can act on, and it is the shape the
//! editor's first command drops straight into.
//!
//! Version negotiation is not here (GTW-940). The listener thread answers every
//! [`Hello`](QaRequest::Hello) against this host's
//! [`editor_hello_facts()`](super::super::config::editor_hello_facts) and refuses every other
//! request [`NotNegotiated`](QaError::NotNegotiated) until one succeeds, so a `Hello` never
//! reaches this drain — and negotiation no longer depends on the editor running a frame.

use bevy::prelude::*;
use gdtf_net_qa_transport::NetInbox;
use gdtf_qa_protocol::{
    command::{CommandCatalogue, CommandOutcome},
    message::{QaError, QaRequest, QaResponse, ServerNameNet},
};

use crate::net_qa::config::EDITOR_QA_SERVER_NAME;

/// Drain the inbox and answer every buffered request (see the module doc).
pub(in crate::net_qa) fn route_editor_requests(inbox: Res<NetInbox>) {
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        match request {
            QaRequest::Catalogue => {
                responder.reply(QaResponse::Catalogue(CommandCatalogue::new(
                    ServerNameNet::new(EDITOR_QA_SERVER_NAME.to_owned()),
                    Vec::new(),
                )));
            }
            // No command set yet, so every name is unknown — and the reply carries the
            // (empty) list of names this host does offer, which is the same self-correcting
            // shape a typo gets once there are commands to name.
            QaRequest::Run(_) => {
                responder.reply(QaResponse::Outcome(CommandOutcome::Unknown {
                    known: Vec::new(),
                }));
            }
            // Listed EXHAUSTIVELY rather than caught by a `_` wildcard, so a new request
            // variant on the shared wire fails this match to compile and forces a
            // deliberate decision here. A `Hello` that reached this drain got past the only
            // code that answers a handshake, so the frame is wrong for this connection.
            QaRequest::Hello(_) => {
                responder.reply(QaResponse::Error(QaError::Malformed));
            }
        }
    }
}
