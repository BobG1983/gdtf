//! The delete-member system: a delete-button press removes the member from the model AND despawns
//! exactly that row, then re-keys the surviving rows' indices in place (GTW-425 C4 / C5).
//!
//! The delete button carries the member's [`MemberRowIndex`] (the model slot to remove) and a
//! [`MemberRowRef`] (the [`MemberRow`](super::super::components::MemberRow) root to despawn).
//! Removing a model member SHIFTS the indices of every later member down by one, so this system
//! also decrements the [`MemberRowIndex`] of every surviving control whose index was ABOVE the
//! deleted one — keeping the rows MUTATED in step with the model (never rebuilding the list — C5).

use bevy::{prelude::*, ui::Interaction};

use crate::states::running::gang_editor::{
    components::{DeleteMemberButton, MemberRowIndex, MemberRowRef},
    model::EditableGang,
};

/// The press-edge query filter [`delete_member_on_press`] reads — a [`DeleteMemberButton`] whose
/// [`Interaction`] changed this frame. A named alias to keep the system signature under clippy's
/// `type_complexity` gate (the widget-driver precedent).
type PressedDelete = (Changed<Interaction>, With<DeleteMemberButton>);

/// Removes a member + its row when its delete button is PRESSED, then re-keys surviving rows
/// (GTW-425 C4 / C5).
///
/// Reads `Changed<Interaction> == Pressed` on the [`DeleteMemberButton`]s (one delete per press
/// edge — the menu-action precedent). On a press it removes the member at the button's
/// [`MemberRowIndex`] from the model (the C4 model effect); if a member WAS removed it despawns the
/// row root named by the button's [`MemberRowRef`] (the C4 view effect) and decrements the
/// [`MemberRowIndex`] of every surviving control whose index was ABOVE the deleted one, so the
/// remaining rows stay keyed to their (now-shifted) model slots WITHOUT a rebuild (C5). Guarded on
/// the model's presence (state-scoped resource — `bevy-traps.md` #1) and registered
/// `run_if(in_state(RunningState::DebugGangEditor))`. Param-only — the despawn goes through
/// [`Commands`] (`bevy-traps.md` #7).
///
/// The press detection reads `Interaction` + `MemberRowRef` (NOT `MemberRowIndex`) so it does not
/// overlap the `&mut MemberRowIndex` re-key query — the deleted slot is read from that single mut
/// query by the pressed button's entity, keeping the two params disjoint (no B0001 — `bevy-traps`
/// query-conflict rule).
pub(in crate::states::running::gang_editor) fn delete_member_on_press(
    mut commands: Commands,
    pressed: Query<(Entity, &Interaction, &MemberRowRef), PressedDelete>,
    mut indices: Query<&mut MemberRowIndex>,
    model: Option<ResMut<EditableGang>>,
) {
    let Some(mut model) = model else {
        return;
    };
    // The first delete pressed this frame (deletes are one-at-a-time in the UI).
    let Some((button, row)) = pressed.iter().find_map(|(entity, interaction, row_ref)| {
        matches!(interaction, Interaction::Pressed).then_some((entity, **row_ref))
    }) else {
        return;
    };
    // The deleted slot is the pressed button's own MemberRowIndex (read from the single mut query).
    let Some(deleted_index) = indices.get(button).ok().map(|index| **index) else {
        return;
    };
    if !model.remove_member(deleted_index) {
        // Out-of-range (stale) index — nothing removed, leave the rows untouched.
        return;
    }
    // Despawn exactly that row (and all its child controls).
    if let Ok(mut row_entity) = commands.get_entity(row) {
        row_entity.despawn();
    }
    // Re-key the surviving rows in place: every control above the deleted slot shifts down one
    // (one `&mut MemberRowIndex` query over ALL controls — no With/Without split to overlap).
    for mut index in &mut indices {
        if **index > deleted_index {
            *index = MemberRowIndex::new(**index - 1);
        }
    }
}
