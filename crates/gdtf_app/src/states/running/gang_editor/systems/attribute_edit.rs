//! The per-member ATTRIBUTE-edit system: a numeric-field commit on one of a member's eight base
//! attributes updates the model AND LIVE-recomputes that member's readonly derived stats via the
//! GTW-384 pipeline (GTW-428 C2 / C3).
//!
//! On a [`NumericFieldCommitted`]`<f32>` for an [`AttributeField`] it (1) reads the committing
//! field's [`MemberRowIndex`] + [`BaseAttribute`] (skipping any non-attribute numeric field), (2)
//! sets that member's attribute in the [`EditableGang`] model to the (already-clamped) committed
//! value, (3) runs the REAL GTW-384 [`derive_stats`](gdtf_battle_sim::ganger::derive_stats) pipeline over
//! the member's now-current attributes × the [`GangerStatTuning`] weights, and (4) mutates the
//! row's readonly [`DerivedStatText`] nodes IN PLACE to the pipeline output (the ui-mutate rule,
//! C5 — never a respawn). The displayed derived value therefore EQUALS the pipeline output for the
//! live attributes (C3); the derivation is the sim's, never reimplemented here.

use bevy::prelude::*;
use gdtf_battle_sim::{
    ganger::{DerivedStats, derive_stats},
    tuning::GangerStatTuning,
};
use gdtf_ui::NumericFieldCommitted;

use crate::states::running::gang_editor::{
    components::{AttributeField, BaseAttribute, DerivedStat, DerivedStatText, MemberRowIndex},
    model::EditableGang,
    systems::derived_display::format_derived,
};

/// Updates a member's base attribute from a numeric-field commit + LIVE-recomputes its derived
/// stats (GTW-428 C2 / C3).
///
/// Reads [`NumericFieldCommitted`]`<f32>` with a [`MessageReader`] (buffered events are messages —
/// `bevy-traps.md` #4). For each commit it resolves the field's [`MemberRowIndex`] +
/// [`BaseAttribute`] (a commit on any non-attribute `f32` field carries no
/// [`AttributeField`]/[`BaseAttribute`] and is skipped), sets the attribute in the model, then
/// re-derives the member's stats and mutates the matching [`DerivedStatText`] nodes in place.
/// Guarded by the model's presence (`Option<ResMut<…>>` — state-scoped resource, `bevy-traps.md`
/// #1) and the tuning's (`Option<Res<…>>` — an absent tuning falls back to the const-default
/// weights so the recompute still runs). Registered `run_if(in_state(RunningState::DebugGangEditor))`.
pub(in crate::states::running::gang_editor) fn commit_member_attribute(
    mut commits: MessageReader<NumericFieldCommitted<f32>>,
    fields: Query<(&MemberRowIndex, &BaseAttribute), With<AttributeField>>,
    mut displays: Query<(&MemberRowIndex, &DerivedStat, &mut Text), With<DerivedStatText>>,
    model: Option<ResMut<EditableGang>>,
    tuning: Option<Res<GangerStatTuning>>,
) {
    let Some(mut model) = model else {
        return;
    };
    // The GTW-384 derivation weights; an absent resource falls back to the const-default so the
    // live recompute still produces a sensible value (C3).
    let tuning = tuning.as_deref().cloned().unwrap_or_default();
    for commit in commits.read() {
        let Ok((row_index, attribute)) = fields.get(commit.field()) else {
            continue;
        };
        let index = **row_index;
        // The committed value is already clamped to the field's NumericRange (C2).
        model.set_member_attribute(index, *attribute, commit.value().value());

        // Re-derive THIS member's stats through the real GTW-384 pipeline (C3) and mutate the
        // matching readonly displays in place (C5).
        let Some(member) = model.member_at(index) else {
            continue;
        };
        let stats: DerivedStats = derive_stats(&member.attributes(), &tuning);
        for (display_index, stat, mut text) in &mut displays {
            if **display_index != index {
                continue;
            }
            let value = format_derived(&stats, *stat);
            if text.0 != value {
                text.0 = value;
            }
        }
    }
}
