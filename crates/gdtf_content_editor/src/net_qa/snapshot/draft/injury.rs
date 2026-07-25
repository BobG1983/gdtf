//! The INJURY draft's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{count_field, debug_field, text_field};
use crate::{InjuryDraft, WeightingDraft};

/// The INJURY draft's fields.
///
/// INJURY is the one mode that edits TWO records at once — the injury def and the
/// per-category weighting table — so the weighting draft's category / context ride here
/// too, reported as absent when that resource is not present.
pub(super) fn injury_fields(
    draft: &InjuryDraft,
    weighting: Option<&WeightingDraft>,
) -> Vec<EditorDraftFieldView> {
    let def = draft.def();
    let mut fields = vec![
        text_field("key", draft.key()),
        debug_field("name", &def.name),
        debug_field("category", &def.category),
        debug_field("severity", &def.severity),
        count_field("effects", def.effects.len()),
        debug_field("post_heal", &def.post_heal),
    ];
    fields.push(debug_field(
        "weighting_category",
        &weighting.map(WeightingDraft::category),
    ));
    fields.push(debug_field(
        "weighting_context",
        &weighting.map(WeightingDraft::context),
    ));
    fields
}
