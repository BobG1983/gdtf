//! The ATTACHMENT draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{count_field, debug_field, text_field};
use crate::AttachmentDraft;

/// The ATTACHMENT draft's fields: the file-stem name buffer, the item's display name, the
/// slot it mounts in, and how many effects it applies.
pub(super) fn attachment_fields(draft: &AttachmentDraft) -> Vec<EditorDraftFieldView> {
    let spec = draft.spec();
    vec![
        text_field("name", draft.name()),
        debug_field("display_name", &spec.display_name),
        debug_field("slot", &spec.slot),
        count_field("effects", spec.effects.len()),
    ]
}
