//! The editor's request drain (GTW-804).
//!
//! [`route_editor_requests`] drains the [`NetInbox`] every frame in the
//! [`EditorNetQaSystems::Gather`](super::EditorNetQaSystems) band and dispatches each
//! [`QaRequest`].
//!
//! This child wires exactly ONE request:
//! [`Hello`](QaRequest::Hello) version negotiation. Every other request kind is answered
//! [`BadRequest`](QaError::BadRequest) — "malformed or nonsensical in the current state" is
//! the accurate reading for a host that services no such request: the editor runs no battle,
//! has no `AppState`, and its own read / drive requests do not exist yet (GTW-805 / GTW-806 /
//! GTW-808 add them). Rejecting immediately is deliberate — the alternative, leaving a
//! request unanswered, hangs the client until its socket timeout reaps it.

use bevy::prelude::*;
use gdtf_net_qa_transport::{NetInbox, Responder};
use gdtf_qa_protocol::envelope::{
    HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet,
};

use super::config::{EDITOR_QA_PROTOCOL_VERSION, EDITOR_QA_SERVER_NAME};

/// Drain the inbox and answer every buffered request (see the module doc).
pub(super) fn route_editor_requests(inbox: Res<NetInbox>) {
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        match request {
            QaRequest::Hello(client_version) => answer_hello(client_version, responder),
            // Every other kind: the editor host offers no handler for it yet. Listed
            // EXHAUSTIVELY rather than caught by a `_` wildcard, so a new request variant on
            // the shared envelope fails this match to compile and forces a deliberate
            // decision here (the game's wildcard-free classification convention).
            QaRequest::GetAppFlow
            | QaRequest::GetBattleState
            | QaRequest::Inject(_)
            | QaRequest::TakeScreenshot { .. }
            | QaRequest::ScreenshotAfter { .. }
            | QaRequest::GetOutput { .. }
            | QaRequest::StartBattle { .. }
            | QaRequest::StepperControl(_)
            | QaRequest::ActivateMenuItem(_)
            | QaRequest::FocusControl(_) => {
                responder.reply(QaResponse::Error(QaError::BadRequest));
            }
        }
    }
}

/// Negotiate the protocol version: reply the handshake facts on a match, or
/// [`VersionMismatch`](QaError::VersionMismatch) otherwise.
///
/// Exact equality, with no capability handshake — the same negotiation the game performs, so
/// one QA client speaks one version to both hosts.
fn answer_hello(client_version: ProtocolVersion, responder: Responder) {
    if client_version == EDITOR_QA_PROTOCOL_VERSION {
        let facts = HelloFacts::new(
            EDITOR_QA_PROTOCOL_VERSION,
            ServerNameNet::new(EDITOR_QA_SERVER_NAME.to_owned()),
        );
        responder.reply(QaResponse::HelloOk(facts));
    } else {
        responder.reply(QaResponse::Error(QaError::VersionMismatch));
    }
}
