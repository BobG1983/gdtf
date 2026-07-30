//! The assertions over the collected replies (GTW-804; the `Load`-phase ones added in
//! GTW-896).

use gdtf_content_editor::{EDITOR_QA_SERVER_NAME, EditorState};
use gdtf_qa_protocol::{
    envelope::{ProtocolVersion, QaError, QaResponse},
    view::EditorReadinessNet,
};

/// The handshake facts the editor answers a matching client version with.
pub(crate) fn assert_hello_ok(reply: &QaResponse) {
    assert!(
        matches!(
            reply,
            QaResponse::HelloOk(facts)
                if facts.protocol == ProtocolVersion::CURRENT
                    && *facts.server == EDITOR_QA_SERVER_NAME
        ),
        "expected the editor's HelloOk handshake facts, got {reply:?}",
    );
}

/// A client one version ahead is refused — proving real negotiation rather than a canned reply.
pub(crate) fn assert_version_mismatch(reply: &QaResponse) {
    assert!(
        matches!(reply, QaResponse::Error(QaError::VersionMismatch)),
        "expected VersionMismatch for a wrong client version, got {reply:?}",
    );
}

/// A request kind the editor services in no state is refused.
pub(crate) fn assert_unsupported(reply: &QaResponse) {
    assert!(
        matches!(reply, QaResponse::Error(QaError::BadRequest)),
        "expected BadRequest for a request the editor does not service yet, got {reply:?}",
    );
}

/// The frame that answered this reply ran under the editor's `Load` asset pass (GTW-896).
///
/// `what` names the reply, so a failure says which exchange slipped past the transition.
pub(crate) fn assert_answered_during_load(state: Option<&EditorState>, what: &str) {
    assert_eq!(
        state,
        Some(&EditorState::Load),
        "{what} was answered by a frame that was no longer in the editor's Load asset pass, so \
         it proves nothing about the pre-Editing handshake. The editor's own Load → Editing gate \
         waits on ten registries and takes several more frames than this exchange does, so this \
         is a real regression in that lifecycle rather than a frame-budget shortfall",
    );
}

/// The readiness the editor put on the wire agrees with the state the test body read for the
/// same frame (GTW-896).
///
/// The WIRE half of the `Load` bracket: the readiness a client polls and the state machine that
/// produced it are the same fact seen from the two ends of the socket, and this is the one reply
/// kind that carries it.
pub(crate) fn assert_readiness_matches_state(reply: &QaResponse, state: Option<&EditorState>) {
    let QaResponse::EditorQueryOptions(view) = reply else {
        unreachable!("expected an EditorQueryOptions reply to close the window, got {reply:?}");
    };
    let expected = match state {
        Some(EditorState::Load) => EditorReadinessNet::Load,
        Some(EditorState::Editing) => EditorReadinessNet::Editing,
        None => unreachable!("the editor's state machine is missing — MapEditorPlugin never ran"),
    };
    assert_eq!(
        view.readiness, expected,
        "the options reply must report the readiness of the very frame that answered it \
         ({state:?}), never a stale or invented one",
    );
}
