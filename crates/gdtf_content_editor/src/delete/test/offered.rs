//! A delete is offered only on the screen its entry names.

use crate::{
    delete::{
        entries::{
            ARMOR_FAMILY, ATTACHMENT_FAMILY, FIELD_FAMILY, INJURY_FAMILY, MELEE_WEAPON_FAMILY,
            PREFAB_FAMILY, WEIGHTING_FAMILY, armor_delete_entry, attachment_delete_entry,
            field_delete_entry, injury_delete_entry, melee_weapon_delete_entry,
            prefab_delete_entry, weighting_delete_entry,
        },
        offered::entries_offered_on,
        registry::DeleteRegistry,
    },
    mode::{EditorMode, InjurySubTab},
};

// The registry the editor builds, in registration order.
fn registry() -> DeleteRegistry {
    let mut registry = DeleteRegistry::default();
    registry.add(prefab_delete_entry());
    registry.add(weighting_delete_entry());
    registry.add(armor_delete_entry());
    registry.add(melee_weapon_delete_entry());
    registry.add(attachment_delete_entry());
    registry.add(injury_delete_entry());
    registry.add(field_delete_entry());
    registry
}

// The family labels offered on one screen, in registration order.
fn labels_on(mode: EditorMode, sub_tab: Option<InjurySubTab>) -> Vec<String> {
    entries_offered_on(&registry(), mode, sub_tab)
        .into_iter()
        .map(|entry| (**entry.family()).clone())
        .collect()
}

#[test]
fn the_weighting_delete_is_offered_on_the_injury_tables_sub_tab() {
    assert_eq!(
        labels_on(EditorMode::Injury, Some(InjurySubTab::Tables)),
        vec![WEIGHTING_FAMILY.to_owned()],
        "the weighting table is the Tables sub-tab's own record, so its delete is the one \
         offered there",
    );
}

#[test]
fn the_injury_def_delete_is_offered_on_the_injury_def_sub_tab() {
    assert_eq!(
        labels_on(EditorMode::Injury, Some(InjurySubTab::Def)),
        vec![INJURY_FAMILY.to_owned()],
        "the injury def is the Def sub-tab's own record, and the weighting delete belongs to \
         the other sub-tab",
    );
}

#[test]
fn the_prefab_delete_is_offered_on_the_prefab_tab() {
    assert_eq!(
        labels_on(EditorMode::Prefab, None),
        vec![PREFAB_FAMILY.to_owned()],
        "the prefab canvas is where a prefab is deleted from",
    );
}

#[test]
fn each_loadout_delete_is_offered_on_its_own_tab() {
    assert_eq!(
        labels_on(EditorMode::Armor, None),
        vec![ARMOR_FAMILY.to_owned()],
        "armor is deleted from the Armor tab",
    );
    assert_eq!(
        labels_on(EditorMode::MeleeWeapon, None),
        vec![MELEE_WEAPON_FAMILY.to_owned()],
        "a melee weapon is deleted from the Melee weapon tab",
    );
    assert_eq!(
        labels_on(EditorMode::Attachment, None),
        vec![ATTACHMENT_FAMILY.to_owned()],
        "an attachment is deleted from the Attachment tab",
    );
    assert_eq!(
        labels_on(EditorMode::Field, None),
        vec![FIELD_FAMILY.to_owned()],
        "a field def is deleted from the Field tab",
    );
}
