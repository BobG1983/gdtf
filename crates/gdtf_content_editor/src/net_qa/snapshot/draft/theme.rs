//! The THEME draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{count_field, debug_field, text_field};
use crate::ThemeDraft;

/// The THEME draft's fields, in form order. The terrain palette is reported as a COUNT: it
/// is a reference list of UUID keys, and a client asking "what is the form showing" wants
/// its size, not a wire copy of the terrain library.
pub(super) fn theme_fields(draft: &ThemeDraft) -> Vec<EditorDraftFieldView> {
    vec![
        text_field("display_name", draft.display_name()),
        debug_field("key", &draft.key()),
        count_field("terrain", draft.terrain().len()),
        debug_field("default_floor", &draft.default_floor()),
    ]
}
