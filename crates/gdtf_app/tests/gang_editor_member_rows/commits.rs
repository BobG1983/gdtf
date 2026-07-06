//! Model mutations through the row controls: name/weapon/armor commits, delete,
//! and per-row isolation.

use bevy::{ecs::entity::Entity, ui::Interaction};
use gdtf_app::test_support::{
    DeleteMemberButton, EditableGang, MemberArmorDropdown, MemberNameField, MemberRow,
    MemberWeaponDropdown,
};
use gdtf_battle_sim::{armor::ArmorName, weapon::WeaponName};
use gdtf_ui::{CommittedTextValue, DropdownSelectionChanged, TextFieldCommitted};

use super::harness::*;

/// C3: a real `TextFieldCommitted` on a member's name field edits the model name.
///
/// Pin: if the commit listener drops the row-index mapping or never sets the name, the model name
/// stays at the default and the assert fails. The name field itself shows the committed value
/// (there is no separate echo node anymore — GTW-499 C1).
#[test]
fn commit_member_name_updates_model() {
    let mut app = editor_app();
    press_add_member(&mut app);
    let field = control_for_row::<MemberNameField>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    let committed = "Razor";
    app.world_mut().write_message(TextFieldCommitted::new(
        field,
        CommittedTextValue::new(committed),
    ));
    app.update();

    let model_name = app
        .world()
        .get_resource::<EditableGang>()
        .and_then(|model| model.member_at(0).map(|m| m.name().as_str().to_owned()));
    assert_eq!(
        model_name.as_deref(),
        Some(committed),
        "a commit on a member's name field must set THAT member's name in the model (C3)",
    );
}

/// C2: a real weapon `DropdownSelectionChanged` edits the model weapon.
///
/// Pin: a dropped selection leaves the model weapon unchanged (default empty) — fails the assert.
/// The dropdown's own label shows the chosen key (there is no separate echo node — GTW-499 C1).
#[test]
fn commit_member_weapon_updates_model() {
    let mut app = editor_app();
    press_add_member(&mut app);
    let control =
        control_for_row::<MemberWeaponDropdown>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    let chosen = WeaponName::new(WEAPON_B.to_owned());
    app.world_mut()
        .write_message(DropdownSelectionChanged::new(control, chosen));
    app.update();

    let model_weapon = app
        .world()
        .get_resource::<EditableGang>()
        .and_then(|model| model.member_at(0).map(|m| (**m.weapon()).clone()));
    assert_eq!(
        model_weapon.as_deref(),
        Some(WEAPON_B),
        "a weapon dropdown selection must set THAT member's weapon key in the model (C2)",
    );
}

/// C2: a real armor `DropdownSelectionChanged` edits the model armor (the armor mirror of
/// [`commit_member_weapon_updates_model`]).
#[test]
fn commit_member_armor_updates_model() {
    let mut app = editor_app();
    press_add_member(&mut app);
    let control =
        control_for_row::<MemberArmorDropdown>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    let chosen = ArmorName::new(ARMOR_B.to_owned());
    app.world_mut()
        .write_message(DropdownSelectionChanged::new(control, chosen));
    app.update();

    let model_armor = app
        .world()
        .get_resource::<EditableGang>()
        .and_then(|model| model.member_at(0).map(|m| (**m.armor()).clone()));
    assert_eq!(
        model_armor.as_deref(),
        Some(ARMOR_B),
        "an armor dropdown selection must set THAT member's armor key in the model (C2)",
    );
}

/// C4: a delete press removes the member from the model AND despawns its row.
///
/// Pin: a no-op delete leaves the model count + row count unchanged — either fails.
#[test]
fn delete_member_removes_from_model_and_list() {
    let mut app = editor_app();
    press_add_member(&mut app);
    press_add_member(&mut app);

    let model_before = app
        .world()
        .get_resource::<EditableGang>()
        .map(|model| model.members().len())
        .unwrap_or_default();
    let rows_before = count_with::<MemberRow>(&mut app);
    assert!(
        model_before >= 2,
        "precondition: at least two members to delete one"
    );

    // Delete the member at row index 0 through its real delete button + the real delete system.
    let delete = control_for_row::<DeleteMemberButton>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(delete) {
        *interaction = Interaction::Pressed;
    }
    app.update();
    app.update();

    let model_after = app
        .world()
        .get_resource::<EditableGang>()
        .map(|model| model.members().len())
        .unwrap_or_default();
    let rows_after = count_with::<MemberRow>(&mut app);
    assert_eq!(
        model_after,
        model_before - 1,
        "delete must remove exactly one member from the model (C4)",
    );
    assert_eq!(
        rows_after,
        rows_before - 1,
        "delete must despawn exactly one member row (C4)",
    );
}

/// C5: editing ONE member leaves the OTHER rows' entity ids unchanged (proves a name/weapon/armor
/// edit MUTATES in place and never rebuilds the whole list).
///
/// Pin: if an edit rebuilt the list (despawn + respawn), the surviving member rows would get NEW
/// entity ids and the set-equality assert would fail.
#[test]
fn editing_one_member_leaves_other_rows_untouched() {
    let mut app = editor_app();
    press_add_member(&mut app);
    press_add_member(&mut app);

    // The row entity at index 1 (the member we will NOT edit).
    let other_row_before = control_for_row::<MemberRow>(&mut app, 1).unwrap_or(Entity::PLACEHOLDER);
    let all_rows_before: std::collections::BTreeSet<Entity> =
        all_with::<MemberRow>(&mut app).into_iter().collect();

    // Edit member 0's name via the real commit path.
    let field = control_for_row::<MemberNameField>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut().write_message(TextFieldCommitted::new(
        field,
        CommittedTextValue::new("Mutated"),
    ));
    app.update();

    let other_row_after = control_for_row::<MemberRow>(&mut app, 1).unwrap_or(Entity::PLACEHOLDER);
    let all_rows_after: std::collections::BTreeSet<Entity> =
        all_with::<MemberRow>(&mut app).into_iter().collect();

    assert_eq!(
        other_row_after, other_row_before,
        "editing member 0 must NOT change the OTHER row's entity id (no whole-list rebuild — C5)",
    );
    assert_eq!(
        all_rows_after, all_rows_before,
        "editing one member must leave EVERY row's entity id unchanged (mutate-not-rebuild — C5)",
    );
}
