//! The editor's request drain (GTW-804).
//!
//! [`route_editor_requests`] drains the [`NetInbox`] every frame in the
//! [`EditorNetQaSystems::Gather`](super::EditorNetQaSystems) band and dispatches each
//! [`QaRequest`].
//!
//! It wires FOUR requests: [`Hello`](QaRequest::Hello) version negotiation, the editor
//! family's ADR 0007 query pair —
//! [`GetEditorQueryOptions`](QaRequest::GetEditorQueryOptions) (which topics are live right
//! now) and [`QueryEditor`](QaRequest::QueryEditor) (one topic's answer), both served from
//! the [`snapshot`](super::snapshot) service (GTW-805) — and
//! [`TakeScreenshot`](QaRequest::TakeScreenshot) (GTW-880), the one request this drain does
//! NOT answer itself: it queues onto the shared transport's
//! [`PendingQueue`](gdtf_net_qa_transport::PendingQueue) and the
//! [`screenshot`](super::screenshot) pump replies once the PNG has landed on disk. Every
//! other request kind is answered [`BadRequest`](QaError::BadRequest) — "malformed or
//! nonsensical in the current state" is the accurate reading for a host that services no such
//! request: the editor runs no battle, has no `AppState`, and its drive requests do not exist
//! yet (GTW-806 adds them). Rejecting immediately is deliberate — the alternative, leaving a
//! request unanswered, hangs the client until its socket timeout reaps it.
//!
//! A `QueryEditor` naming a topic the editor is not servicing right now — during the `Load`
//! asset pass, the topics backed by an `Editing`-scoped model resource — is answered
//! `BadRequest` too, never a fabricated empty view. Which topics those are is the
//! availability filter's call, not a fixed list: see
//! [`topic_available`](super::snapshot::topic_available). The client polls
//! `GetEditorQueryOptions` first; it is answerable in EVERY editor state and carries the
//! `Load` / `Editing` readiness, so an inject loop can wait for the asset pass instead of
//! racing it.

use bevy::prelude::*;
use gdtf_net_qa_transport::{NetInbox, PendingQueue, Responder};
use gdtf_qa_protocol::{
    envelope::{HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet},
    view::EditorQueryKind,
};

use super::{
    config::{EDITOR_QA_PROTOCOL_VERSION, EDITOR_QA_SERVER_NAME},
    screenshot::EditorScreenshotPayload,
    snapshot::{EditorQaModel, answer_topic, options_view, topic_available},
};

/// Drain the inbox and answer every buffered request (see the module doc).
pub(super) fn route_editor_requests(
    inbox: Res<NetInbox>,
    model: EditorQaModel,
    mut screenshots: ResMut<PendingQueue<EditorScreenshotPayload>>,
) {
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        match request {
            QaRequest::Hello(client_version) => answer_hello(client_version, responder),
            QaRequest::GetEditorQueryOptions => {
                responder.reply(QaResponse::EditorQueryOptions(options_view(&model)));
            }
            QaRequest::QueryEditor(kind) => answer_editor_query(kind, &model, responder),
            // Deferred, never answered here: the capture pump claims this the SAME frame
            // (it runs `.after` this drain) and replies only once the PNG lands on disk.
            QaRequest::TakeScreenshot { name } => {
                screenshots.push_new(EditorScreenshotPayload::new(name), responder);
            }
            // Every other kind: the editor host offers no handler for it. Listed
            // EXHAUSTIVELY rather than caught by a `_` wildcard, so a new request variant on
            // the shared envelope fails this match to compile and forces a deliberate
            // decision here (the game's wildcard-free classification convention).
            QaRequest::GetAppFlow
            | QaRequest::GetBattleState
            | QaRequest::Inject(_)
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

/// Answer one topic query: the topic's view when the editor is servicing it, or
/// [`BadRequest`](QaError::BadRequest) when it is not.
///
/// The availability check and the answer run off the SAME model read, so a topic the options
/// reply just advertised is the topic this answers — and one it withheld is refused rather
/// than answered with an invented empty view.
fn answer_editor_query(kind: EditorQueryKind, model: &EditorQaModel, responder: Responder) {
    let reply = if topic_available(kind, model) {
        answer_topic(kind, model)
    } else {
        None
    };
    match reply {
        Some(reply) => responder.reply(QaResponse::EditorQuery(reply)),
        None => responder.reply(QaResponse::Error(QaError::BadRequest)),
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
