use gdtf_content_editor::{EDITOR_QA_SERVER_NAME, EditorState};
use gdtf_qa_protocol::{
    command::CommandOutcome,
    message::{ProtocolVersion, QaError, QaResponse},
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

pub(crate) fn assert_version_mismatch(reply: &QaResponse) {
    assert!(
        matches!(reply, QaResponse::Error(QaError::VersionMismatch)),
        "expected VersionMismatch for a wrong client version, got {reply:?}",
    );
}

pub(crate) fn assert_unknown_command(reply: &QaResponse) {
    let QaResponse::Outcome(CommandOutcome::Unknown { known }) = reply else {
        unreachable!(
            "expected an Unknown outcome for a command the editor has not built, got {reply:?}"
        );
    };
    assert!(
        known.is_empty(),
        "the editor offers no commands yet, so the known list is empty: {known:?}",
    );
}

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

pub(crate) fn assert_editor_catalogue(reply: &QaResponse, state: Option<&EditorState>) {
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("expected a Catalogue reply to close the window, got {reply:?}");
    };
    assert_eq!(
        *catalogue.host, EDITOR_QA_SERVER_NAME,
        "the catalogue must answer under the EDITOR's own host name, never the game's",
    );
    assert!(
        catalogue.entries.is_empty(),
        "the editor publishes no commands yet: {catalogue:?}",
    );
    assert!(
        state.is_some(),
        "the editor's state machine must exist on the frame that answered — MapEditorPlugin \
         never ran otherwise",
    );
}
