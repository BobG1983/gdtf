//! The WEAPON draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{count_field, debug_field, text_field};
use crate::WeaponDraft;

/// The ranged WEAPON draft's fields: the name buffer plus the spec's headline stats,
/// vocabularies and lists.
pub(super) fn weapon_fields(draft: &WeaponDraft) -> Vec<EditorDraftFieldView> {
    let spec = draft.spec();
    vec![
        text_field("name", draft.name()),
        debug_field("damage", &spec.damage),
        debug_field("punch", &spec.punch),
        debug_field("shred", &spec.shred),
        debug_field("damage_type", &spec.damage_type),
        debug_field("accuracy", &spec.accuracy),
        debug_field("base_spread", &spec.base_spread),
        debug_field("kickback", &spec.kickback),
        debug_field("magazine", &spec.magazine),
        debug_field("accepts", &spec.accepts),
        debug_field("fire_mode", &spec.fire_mode),
        debug_field("handedness", &spec.handedness),
        debug_field("trajectory", &spec.trajectory),
        count_field("attachments", spec.attachments.len()),
    ]
}
