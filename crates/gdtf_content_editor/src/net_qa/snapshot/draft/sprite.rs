//! The SPRITE draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{debug_field, text_field};
use crate::SpriteDraft;

/// The SPRITE draft's fields: the name, the base source and anchor, and whether the sprite
/// carries per-facing overrides / an animation.
pub(super) fn sprite_fields(draft: &SpriteDraft) -> Vec<EditorDraftFieldView> {
    let def = draft.def();
    vec![
        text_field("name", draft.name()),
        debug_field("source", &def.source),
        debug_field("anchor", &def.anchor),
        debug_field("facings", &def.facings.is_some()),
        debug_field("animation", &def.animation.is_some()),
    ]
}
