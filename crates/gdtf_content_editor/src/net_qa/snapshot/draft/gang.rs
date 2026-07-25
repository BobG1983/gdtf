//! The GANG draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{count_field, text_field};
use crate::GangDraft;

/// The GANG draft's fields: the roster name and how many members it holds.
pub(super) fn gang_fields(draft: &GangDraft) -> Vec<EditorDraftFieldView> {
    vec![
        text_field("name", draft.name()),
        count_field("members", draft.members().len()),
    ]
}
