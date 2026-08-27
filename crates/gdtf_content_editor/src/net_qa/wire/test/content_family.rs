use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    EditorMode,
    net_qa::wire::{ContentFamilyNet, EditorModeNet},
};

// The mode tab each family names, so the two vocabularies can be compared by name.
const TABS: [EditorModeNet; 9] = [
    EditorModeNet::Weapon,
    EditorModeNet::MeleeWeapon,
    EditorModeNet::Armor,
    EditorModeNet::Gang,
    EditorModeNet::Terrain,
    EditorModeNet::Theme,
    EditorModeNet::Injury,
    EditorModeNet::Sprite,
    EditorModeNet::Attachment,
];

#[test]
fn every_content_family_arm_round_trips() {
    for family in ContentFamilyNet::ALL {
        assert_ron_round_trip(&family);
    }
}

#[test]
fn the_content_family_traces_a_usable_shape() {
    assert_schema_is_usable::<ContentFamilyNet>("ContentFamilyNet");
}

#[test]
fn every_family_is_listed_once() {
    let mut seen: Vec<String> = Vec::with_capacity(ContentFamilyNet::ALL.len());
    for family in ContentFamilyNet::ALL {
        let named = format!("{family:?}");
        assert!(
            !seen.contains(&named),
            "{named} appears twice in ContentFamilyNet::ALL, so one family would be tallied \
             under two slots and another under none",
        );
        seen.push(named);
    }
}

#[test]
fn every_family_is_spelled_the_way_its_mode_tab_is() {
    for (family, tab) in ContentFamilyNet::ALL.into_iter().zip(TABS) {
        assert_eq!(
            format!("{family:?}"),
            format!("{tab:?}"),
            "a client uses one family vocabulary across editor.families and wait, so {family:?} \
             must read on the wire exactly as the mode tab it names does",
        );
    }
}

#[test]
fn the_prefab_and_field_tabs_name_no_family() {
    let named: Vec<String> = ContentFamilyNet::ALL
        .into_iter()
        .map(|family| format!("{family:?}"))
        .collect();
    for absent in [EditorModeNet::Prefab, EditorModeNet::Field] {
        assert!(
            !named.contains(&format!("{absent:?}")),
            "{absent:?} rearms no validation pass — Prefab owns no registry and \
             FieldDefRegistry is not watched — so a wait on it could never come true: {named:?}",
        );
    }
    assert_eq!(
        ContentFamilyNet::ALL.len() + 2,
        EditorMode::TAB_ORDER.len(),
        "every mode tab but Prefab and Field owns a watched registry, so a new tab nobody \
         classified is missing from ContentFamilyNet::ALL",
    );
}
