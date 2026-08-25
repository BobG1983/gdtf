use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    net_qa::wire::{EditorDraftOutcomeNet, EditorDraftRonNet, EditorSaveFaultNet},
    save_record::EditorSaveFault,
};

fn projected() -> EditorDraftRonNet {
    EditorDraftRonNet::new("(\n    display_name: \"Ash Wall\",\n)".to_owned())
}

#[test]
fn projected_ron_text_round_trips() {
    assert_ron_round_trip(&projected());
}

#[test]
fn both_draft_outcomes_round_trip() {
    assert_ron_round_trip(&EditorDraftOutcomeNet::Ron(projected()));
    assert_ron_round_trip(&EditorDraftOutcomeNet::NotSavable(
        EditorSaveFaultNet::from_fault(&EditorSaveFault::MissingMountedWeapon),
    ));
}

#[test]
fn a_projected_draft_keeps_the_text_it_wrapped() {
    let text = "(\n    display_name: \"Ash Wall\",\n)".to_owned();
    assert_eq!(
        *EditorDraftRonNet::new(text.clone()),
        text,
        "the wire text is the writer's own text, so a client can compare it to a file it reads",
    );
}

#[test]
fn the_draft_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorDraftRonNet>("EditorDraftRonNet");
    assert_schema_is_usable::<EditorDraftOutcomeNet>("EditorDraftOutcomeNet");
}
