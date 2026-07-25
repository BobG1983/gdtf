//! The topic-availability filter + the options reply (GTW-805).
//!
//! ADR 0007's live discovery signal for the editor family: the SAME predicate decides both
//! what [`GetEditorQueryOptions`](gdtf_qa_protocol::envelope::QaRequest::GetEditorQueryOptions)
//! advertises and whether a
//! [`QueryEditor`](gdtf_qa_protocol::envelope::QaRequest::QueryEditor) is answered, so the two
//! can never disagree — the game router's `request_available` pattern, applied to
//! [`EditorQueryKind`] instead of `RequestKindNet`.

use gdtf_qa_protocol::view::{EditorQueryKind, EditorQueryOptionsView, EditorQueryTopicView};

use super::model::EditorQaModel;

/// Whether the editor will answer `kind` right now.
///
/// A wildcard-free `match` (ADR 0007's drift guard): a topic added to [`EditorQueryKind`]
/// fails this match to compile until it states when it is live, so the static topic list and
/// the live-availability list cannot drift apart.
///
/// Readiness is always answerable — it is what a client polls DURING the editor's `Load`
/// asset pass to learn when the editor is ready. Every other topic reads a model resource
/// that is scoped to `Editing`, so each is offered exactly when its own resource is present:
/// a topic is never advertised unless the answer really can be produced.
pub(in crate::net_qa) fn topic_available(kind: EditorQueryKind, model: &EditorQaModel) -> bool {
    match kind {
        EditorQueryKind::Readiness => true,
        EditorQueryKind::Mode => model.mode().is_some(),
        EditorQueryKind::Session => model.session().is_some(),
        EditorQueryKind::Draft => model.mode().is_some_and(|mode| model.drafts().has(mode)),
        EditorQueryKind::Validation => model.report().is_some(),
    }
}

/// The options reply: the editor's readiness plus every topic it will service right now, in
/// topic-declaration order.
pub(in crate::net_qa) fn options_view(model: &EditorQaModel) -> EditorQueryOptionsView {
    let topics = EditorQueryKind::ALL
        .into_iter()
        .filter(|kind| topic_available(*kind, model))
        .map(EditorQueryTopicView::offered)
        .collect();
    EditorQueryOptionsView::new(model.readiness(), topics)
}
