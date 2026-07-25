//! [`draft_view`] — the active mode's draft snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftView;

use super::model::EditorDraftModel;
use crate::{EditorMode, net_qa::snapshot::mode::mode_to_net};

/// The DRAFT topic's answer: the active mode's draft fields, tagged with the mode that owns
/// them — or [`None`] when that mode's draft resource is absent (the editor's `Load` pass),
/// which is exactly when the topic is not offered.
pub(in crate::net_qa) fn draft_view(
    mode: EditorMode,
    drafts: &EditorDraftModel,
) -> Option<EditorDraftView> {
    let fields = drafts.fields(mode)?;
    Some(EditorDraftView::new(mode_to_net(mode), fields))
}
