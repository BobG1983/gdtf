//! [`answer_topic`] — one topic's answer, built from the editor's model (GTW-805).

use gdtf_qa_protocol::view::{EditorQueryKind, EditorQueryReply, EditorQueryView};

use super::{
    draft::draft_view, mode::mode_view, model::EditorQaModel, session::session_view,
    validation::validation_view,
};

/// Build the reply to a [`QueryEditor`](gdtf_qa_protocol::envelope::QaRequest::QueryEditor),
/// or [`None`] when the topic's model resource is absent — which is exactly when
/// [`topic_available`](super::topics::topic_available) refuses to advertise it, so the
/// caller answers that case [`BadRequest`](gdtf_qa_protocol::envelope::QaError::BadRequest)
/// rather than fabricating an empty view.
///
/// A wildcard-free `match` over [`EditorQueryKind`]: a new topic must say what it answers
/// with. Every reply carries the editor's readiness alongside the topic's answer.
pub(in crate::net_qa) fn answer_topic(
    kind: EditorQueryKind,
    model: &EditorQaModel,
) -> Option<EditorQueryReply> {
    let readiness = model.readiness();
    let view = match kind {
        EditorQueryKind::Readiness => EditorQueryView::Readiness(readiness),
        EditorQueryKind::Mode => EditorQueryView::Mode(mode_view(model.mode()?)),
        EditorQueryKind::Session => EditorQueryView::Session(session_view(model.session()?)),
        EditorQueryKind::Draft => {
            EditorQueryView::Draft(draft_view(model.mode()?, model.drafts())?)
        }
        EditorQueryKind::Validation => {
            EditorQueryView::Validation(validation_view(model.report()?, model.checks_complete()))
        }
    };
    Some(EditorQueryReply::new(readiness, view))
}
