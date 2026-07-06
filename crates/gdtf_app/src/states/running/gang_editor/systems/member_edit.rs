//! Per-member inline-edit systems: name field commit + weapon / armor dropdown selection
//! (GTW-425 C2 / C3).
//!
//! Each row control carries the member's [`MemberRowIndex`]. A commit / selection on a control
//! mutates that member in the [`EditableGang`](super::super::model::EditableGang) model — never
//! rebuilding the list. The editing control itself shows its own value (the text field's text, the
//! dropdown's own label that `select_dropdown_option` mutates in place), so there is no separate
//! echo node to keep in sync (GTW-499 C1: exactly one control per field).

use bevy::prelude::*;
use gdtf_battle_sim::{armor::ArmorName, ganger::GangerName, weapon::WeaponName};
use gdtf_ui::{DropdownSelectionChanged, TextFieldCommitted};

use crate::states::running::gang_editor::{
    components::{MemberArmorDropdown, MemberNameField, MemberRowIndex, MemberWeaponDropdown},
    model::EditableGang,
};

/// Updates a MEMBER's name from a commit on its inline name field (GTW-425 C3).
///
/// Reads [`TextFieldCommitted`] with a [`MessageReader`] (buffered events are messages —
/// `bevy-traps.md` #4). For each commit it looks up the committing field's [`MemberRowIndex`]
/// (skipping a commit on any non-member field, e.g. the gang-name field, which carries no row
/// index) and sets that member's name in the model. The name field itself shows the committed
/// value — there is no separate echo node to mutate (GTW-499 C1). Guarded by the model's presence
/// (`Option<ResMut<…>>` — state-scoped resource, `bevy-traps.md` #1) and registered
/// `run_if(in_state(RunningState::DebugGangEditor))`.
pub(in crate::states::running::gang_editor) fn commit_member_name(
    mut commits: MessageReader<TextFieldCommitted>,
    fields: Query<&MemberRowIndex, With<MemberNameField>>,
    model: Option<ResMut<EditableGang>>,
) {
    let Some(mut model) = model else {
        return;
    };
    for commit in commits.read() {
        let Ok(row_index) = fields.get(commit.field()) else {
            continue;
        };
        let index = **row_index;
        let value = commit.value().value().to_owned();
        model.set_member_name(index, GangerName::new(value));
    }
}

/// Updates a MEMBER's weapon key from a weapon-dropdown selection (GTW-425 C2).
///
/// Reads [`DropdownSelectionChanged`]`<`[`WeaponName`]`>` (only weapon dropdowns emit this `T`, so
/// an armor selection never reaches here). For each it resolves the changed control's
/// [`MemberRowIndex`] and sets that member's weapon in the model. The dropdown's own label already
/// shows the chosen key (`select_dropdown_option` mutates it in place), so there is no echo node to
/// keep in sync (GTW-499 C1). Guarded + registered as [`commit_member_name`].
pub(in crate::states::running::gang_editor) fn commit_member_weapon(
    mut changes: MessageReader<DropdownSelectionChanged<WeaponName>>,
    controls: Query<&MemberRowIndex, With<MemberWeaponDropdown>>,
    model: Option<ResMut<EditableGang>>,
) {
    let Some(mut model) = model else {
        return;
    };
    for change in changes.read() {
        let Ok(row_index) = controls.get(change.control()) else {
            continue;
        };
        let index = **row_index;
        model.set_member_weapon(index, change.id().clone());
    }
}

/// Updates a MEMBER's armor key from an armor-dropdown selection (GTW-425 C2). The armor mirror of
/// [`commit_member_weapon`].
pub(in crate::states::running::gang_editor) fn commit_member_armor(
    mut changes: MessageReader<DropdownSelectionChanged<ArmorName>>,
    controls: Query<&MemberRowIndex, With<MemberArmorDropdown>>,
    model: Option<ResMut<EditableGang>>,
) {
    let Some(mut model) = model else {
        return;
    };
    for change in changes.read() {
        let Ok(row_index) = controls.get(change.control()) else {
            continue;
        };
        let index = **row_index;
        model.set_member_armor(index, change.id().clone());
    }
}
