//! The ARMOR draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{debug_field, text_field};
use crate::ArmorDraft;

/// The ARMOR draft's fields: the suit name and each of its six body-part pieces.
pub(super) fn armor_fields(draft: &ArmorDraft) -> Vec<EditorDraftFieldView> {
    let spec = draft.spec();
    vec![
        text_field("name", draft.name()),
        debug_field("head", &spec.head),
        debug_field("torso", &spec.torso),
        debug_field("left_arm", &spec.left_arm),
        debug_field("right_arm", &spec.right_arm),
        debug_field("left_leg", &spec.left_leg),
        debug_field("right_leg", &spec.right_leg),
    ]
}
