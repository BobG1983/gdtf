//! The MELEE-WEAPON draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{count_field, debug_field, text_field};
use crate::MeleeWeaponDraft;

/// The MELEE-WEAPON draft's fields: the name buffer plus the spec's shared damage group and
/// its melee-only reach / fight-mode / shove.
pub(super) fn melee_fields(draft: &MeleeWeaponDraft) -> Vec<EditorDraftFieldView> {
    let spec = draft.spec();
    vec![
        text_field("name", draft.name()),
        debug_field("damage", &spec.damage),
        debug_field("punch", &spec.punch),
        debug_field("shred", &spec.shred),
        debug_field("damage_type", &spec.damage_type),
        debug_field("fatal_bias", &spec.fatal_bias),
        debug_field("handedness", &spec.handedness),
        debug_field("reach", &spec.reach),
        debug_field("fight_mode", &spec.fight_mode),
        debug_field("shove", &spec.shove),
        count_field("attachments", spec.attachments.len()),
    ]
}
