//! The per-phase assertions over the collected replies (GTW-805).

use gdtf_qa_protocol::{
    envelope::{ProtocolVersion, QaError, QaResponse},
    view::{EditorModeNet, EditorQueryKind, EditorQueryView, EditorReadinessNet},
};

use crate::support::{DANGLING_KEY, DRAFT_NAME};

/// The topics an options reply offers, in order.
fn offered_topics(reply: &QaResponse) -> Vec<EditorQueryKind> {
    let QaResponse::EditorQueryOptions(view) = reply else {
        unreachable!("expected an EditorQueryOptions reply, got {reply:?}");
    };
    view.topics.iter().map(|topic| topic.kind).collect()
}

/// The `Load`-phase assertions (see the suite doc).
pub(crate) fn assert_load_phase(replies: &[QaResponse]) {
    let [hello, options, readiness, mode] = replies else {
        unreachable!("expected exactly four Load-phase replies, got {replies:?}");
    };
    assert!(
        matches!(hello, QaResponse::HelloOk(facts) if facts.protocol == ProtocolVersion::CURRENT),
        "expected the editor's HelloOk at the CURRENT protocol version, got {hello:?}",
    );

    let QaResponse::EditorQueryOptions(view) = options else {
        unreachable!("expected an EditorQueryOptions reply, got {options:?}");
    };
    assert_eq!(
        view.readiness,
        EditorReadinessNet::Load,
        "the options reply must report the Load asset pass",
    );
    assert_eq!(
        offered_topics(options),
        vec![EditorQueryKind::Readiness],
        "during Load only the readiness topic is answerable — the model does not exist yet",
    );

    let QaResponse::EditorQuery(reply) = readiness else {
        unreachable!("expected an EditorQuery reply, got {readiness:?}");
    };
    assert_eq!(reply.readiness, EditorReadinessNet::Load);
    assert_eq!(
        reply.view,
        EditorQueryView::Readiness(EditorReadinessNet::Load),
        "the readiness topic is observable DURING Load",
    );

    assert!(
        matches!(mode, QaResponse::Error(QaError::BadRequest)),
        "an unoffered topic must be refused, not answered with an empty view: got {mode:?}",
    );
}

/// The `Editing`-phase assertions (see the suite doc).
pub(crate) fn assert_editing_phase(replies: &[QaResponse]) {
    let [options, rest @ ..] = replies else {
        unreachable!("expected an options reply and one per topic, got {replies:?}");
    };
    assert_eq!(
        offered_topics(options),
        EditorQueryKind::ALL.to_vec(),
        "in Editing every topic in the EditorQueryKind enum is offered — the options list \
         and the enum stay in step",
    );
    assert_eq!(rest.len(), EditorQueryKind::ALL.len());

    for (kind, reply) in EditorQueryKind::ALL.into_iter().zip(rest) {
        let QaResponse::EditorQuery(reply) = reply else {
            unreachable!("expected an EditorQuery reply for {kind:?}, got {reply:?}");
        };
        assert_eq!(
            reply.readiness,
            EditorReadinessNet::Editing,
            "every editor query reply carries the readiness that produced it",
        );
        assert_topic_view(kind, &reply.view);
    }
}

/// Assert one topic's answer matches the model the fixture inserted.
fn assert_topic_view(kind: EditorQueryKind, view: &EditorQueryView) {
    match (kind, view) {
        (EditorQueryKind::Readiness, EditorQueryView::Readiness(readiness)) => {
            assert_eq!(*readiness, EditorReadinessNet::Editing);
        }
        (EditorQueryKind::Mode, EditorQueryView::Mode(mode)) => {
            assert_eq!(mode.active, EditorModeNet::Terrain);
            assert_eq!(*mode.label, "TERRAIN");
            assert_eq!(*mode.tab_index, 0);
        }
        (EditorQueryKind::Session, EditorQueryView::Session(session)) => {
            assert_eq!(*session.grid_size.width, 12);
            assert_eq!(*session.grid_size.height, 9);
            assert_eq!(*session.grid_size.levels, 2);
            assert!(
                session.default_floor.is_some(),
                "the session's resolved default floor must ride the view",
            );
            assert!(session.selected_tile.is_none());
        }
        (EditorQueryKind::Draft, EditorQueryView::Draft(draft)) => {
            assert_eq!(draft.mode, EditorModeNet::Terrain);
            let Some(name) = draft
                .fields
                .iter()
                .find(|field| *field.name == "display_name")
            else {
                unreachable!("the TERRAIN draft must report display_name: {draft:?}");
            };
            assert_eq!(*name.value, DRAFT_NAME);
        }
        (EditorQueryKind::Validation, EditorQueryView::Validation(validation)) => {
            assert!(
                !*validation.checks_complete,
                "the checks have not run in this fixture",
            );
            let [finding] = validation.findings.as_slice() else {
                unreachable!("expected the one recorded finding, got {validation:?}");
            };
            assert!(
                finding.detail.contains(DANGLING_KEY),
                "the finding detail must name the dangling key, got {:?}",
                finding.detail,
            );
        }
        (kind, view) => unreachable!("{kind:?} was answered with the wrong view: {view:?}"),
    }
}
