//! Per-member inline-edit systems: name field commit + weapon / armor dropdown selection
//! (GTW-425 C2 / C3).
//!
//! Each row control carries the member's [`MemberRowIndex`]. A commit / selection on a control
//! mutates that member in the [`EditableGang`](super::super::model::EditableGang) model AND
//! mutates the row's matching echo [`Text`](bevy::prelude::Text) node IN PLACE (the
//! ui-mutate-not-respawn rule, C5) — never rebuilding the list. The text node is found by reading
//! the editing control's `MemberRowIndex` and matching it against the sibling text node carrying
//! the SAME index.

use bevy::prelude::*;
use gdtf_battle_sim::{ArmorName, GangerName, WeaponName};
use gdtf_ui::{DropdownSelectionChanged, TextFieldCommitted};

use crate::states::running::editor::{
    components::{
        MemberArmorDropdown, MemberArmorText, MemberNameField, MemberNameText, MemberRowIndex,
        MemberWeaponDropdown, MemberWeaponText,
    },
    model::EditableGang,
};

/// Updates a MEMBER's name from a commit on its inline name field, and mutates the row's name echo
/// text in place (GTW-425 C3 / C5).
///
/// Reads [`TextFieldCommitted`] with a [`MessageReader`] (buffered events are messages —
/// `bevy-traps.md` #4). For each commit it looks up the committing field's [`MemberRowIndex`]
/// (skipping a commit on any non-member field, e.g. the gang-name field, which carries no row
/// index), sets that member's name in the model, then mutates the [`MemberNameText`] node carrying
/// the same index. Guarded by the model's presence (`Option<ResMut<…>>` — state-scoped resource,
/// `bevy-traps.md` #1) and registered `run_if(in_state(RunningState::DebugEditor))`.
pub(in crate::states::running::editor) fn commit_member_name(
    mut commits: MessageReader<TextFieldCommitted>,
    fields: Query<&MemberRowIndex, With<MemberNameField>>,
    mut name_texts: Query<(&MemberRowIndex, &mut Text), With<MemberNameText>>,
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
        model.set_member_name(index, GangerName::new(value.clone()));
        // Mutate the matching row's name echo text in place (C5).
        for (text_index, mut text) in &mut name_texts {
            if **text_index == index && text.0 != value {
                text.0.clone_from(&value);
            }
        }
    }
}

/// Updates a MEMBER's weapon key from a weapon-dropdown selection, and mutates the row's weapon
/// echo text in place (GTW-425 C2 / C5).
///
/// Reads [`DropdownSelectionChanged`]`<`[`WeaponName`]`>` (only weapon dropdowns emit this `T`, so
/// an armor selection never reaches here). For each it resolves the changed control's
/// [`MemberRowIndex`], sets that member's weapon in the model, then mutates the
/// [`MemberWeaponText`] node carrying the same index. Guarded + registered as
/// [`commit_member_name`].
pub(in crate::states::running::editor) fn commit_member_weapon(
    mut changes: MessageReader<DropdownSelectionChanged<WeaponName>>,
    controls: Query<&MemberRowIndex, With<MemberWeaponDropdown>>,
    mut weapon_texts: Query<(&MemberRowIndex, &mut Text), With<MemberWeaponText>>,
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
        let weapon = change.id().clone();
        model.set_member_weapon(index, weapon.clone());
        let label = (*weapon).clone();
        for (text_index, mut text) in &mut weapon_texts {
            if **text_index == index && text.0 != label {
                text.0.clone_from(&label);
            }
        }
    }
}

/// Updates a MEMBER's armor key from an armor-dropdown selection, and mutates the row's armor echo
/// text in place (GTW-425 C2 / C5). The armor mirror of [`commit_member_weapon`].
pub(in crate::states::running::editor) fn commit_member_armor(
    mut changes: MessageReader<DropdownSelectionChanged<ArmorName>>,
    controls: Query<&MemberRowIndex, With<MemberArmorDropdown>>,
    mut armor_texts: Query<(&MemberRowIndex, &mut Text), With<MemberArmorText>>,
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
        let armor = change.id().clone();
        model.set_member_armor(index, armor.clone());
        let label = (*armor).clone();
        for (text_index, mut text) in &mut armor_texts {
            if **text_index == index && text.0 != label {
                text.0.clone_from(&label);
            }
        }
    }
}
