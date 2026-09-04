use std::path::PathBuf;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    EditorMode,
    mcp::wire::{EditorLastSaveRowNet, EditorModeNet, LastSaveOutcomeNet},
    save_record::{EditorSaveFault, SaveOutcome, SavedAssetPath},
};

fn wrote() -> SaveOutcome {
    SaveOutcome::Wrote(SavedAssetPath::new(PathBuf::from(
        "assets/content/armor/flak_vest.armor.ron",
    )))
}

#[test]
fn both_recorded_outcomes_round_trip() {
    assert_ron_round_trip(&LastSaveOutcomeNet::from_outcome(&wrote()));
    assert_ron_round_trip(&LastSaveOutcomeNet::from_outcome(&SaveOutcome::Failed(
        EditorSaveFault::NoWorkspaceRoot,
    )));
}

#[test]
fn a_row_round_trips() {
    assert_ron_round_trip(&EditorLastSaveRowNet::new(
        EditorModeNet::from_mode(EditorMode::Armor),
        LastSaveOutcomeNet::from_outcome(&wrote()),
    ));
}

#[test]
fn a_written_record_carries_the_path_the_save_wrote() {
    let LastSaveOutcomeNet::Wrote { path } = LastSaveOutcomeNet::from_outcome(&wrote()) else {
        unreachable!("a Wrote outcome mirrors to a Wrote outcome");
    };
    assert_eq!(
        *path, "assets/content/armor/flak_vest.armor.ron",
        "the record names the file the save wrote, which is what tells two saves under one mode \
         apart",
    );
}

#[test]
fn the_last_save_types_trace_usable_shapes() {
    assert_schema_is_usable::<LastSaveOutcomeNet>("LastSaveOutcomeNet");
    assert_schema_is_usable::<EditorLastSaveRowNet>("EditorLastSaveRowNet");
}
