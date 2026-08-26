use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    EditorFamilyEntryNet, EditorFamilyLabelNet, EditorFamilyRowNet, EditorKeyNet, EditorModeNet,
};

fn entry(key: &str, label: &str) -> EditorFamilyEntryNet {
    EditorFamilyEntryNet::new(
        EditorKeyNet::new(key.to_owned()),
        EditorFamilyLabelNet::new(label.to_owned()),
    )
}

#[test]
fn a_family_label_round_trips() {
    assert_ron_round_trip(&EditorFamilyLabelNet::new("Ash Optic".to_owned()));
}

#[test]
fn a_family_entry_round_trips_and_keeps_its_key() {
    let member: EditorFamilyEntryNet = entry("ash_optic", "Ash Optic");
    assert_ron_round_trip(&member);
    assert_eq!(
        **member.key(),
        "ash_optic".to_owned(),
        "the entry hands back the key it was built with, which is what the sort orders on",
    );
}

#[test]
fn a_family_row_round_trips() {
    assert_ron_round_trip(&EditorFamilyRowNet::new(
        EditorModeNet::Armor,
        vec![
            entry("flak_vest", "Flak Vest"),
            entry("mesh_armor", "Mesh Armor"),
        ],
    ));
}

#[test]
fn a_family_row_with_no_members_round_trips() {
    assert_ron_round_trip(&EditorFamilyRowNet::new(EditorModeNet::Injury, Vec::new()));
}

#[test]
fn the_family_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorFamilyLabelNet>("EditorFamilyLabelNet");
    assert_schema_is_usable::<EditorFamilyEntryNet>("EditorFamilyEntryNet");
    assert_schema_is_usable::<EditorFamilyRowNet>("EditorFamilyRowNet");
}
