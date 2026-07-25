//! The TERRAIN draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{count_field, debug_field, text_field};
use crate::TerrainDraft;

/// The TERRAIN draft's fields, in form order.
pub(super) fn terrain_fields(draft: &TerrainDraft) -> Vec<EditorDraftFieldView> {
    vec![
        text_field("display_name", draft.display_name()),
        debug_field("kind", &draft.kind()),
        debug_field("cover_hp", &draft.cover_hp()),
        debug_field("slab_hp", &draft.slab_hp()),
        debug_field("armor_protection", &draft.armor_protection()),
        debug_field("armor_hardness", &draft.armor_hardness()),
        debug_field("height_band", &draft.height_band()),
        debug_field("graphic", &draft.graphic()),
        debug_field("footfall", &draft.footfall()),
        debug_field("mounted_weapon", &draft.mounted_weapon()),
        count_field("tags", draft.tags().len()),
        debug_field("blocks_pathing", &draft.blocks_pathing()),
        debug_field("blocks_los", &draft.blocks_los()),
        debug_field("uuid", &draft.uuid()),
    ]
}
