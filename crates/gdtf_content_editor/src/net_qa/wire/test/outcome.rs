use std::path::Path;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    net_qa::wire::{
        EditorKeyNet, EditorLoadOutcomeNet, EditorNewOutcomeNet, EditorRefusalNet,
        EditorSaveFaultNet, EditorSaveOutcomeNet, SavedPathNet,
    },
    save_record::EditorSaveFault,
};

#[test]
fn every_new_outcome_arm_round_trips() {
    assert_ron_round_trip(&EditorNewOutcomeNet::Blanked);
    assert_ron_round_trip(&EditorNewOutcomeNet::Refused(
        EditorRefusalNet::ThemeNewIsUndoneBySync,
    ));
}

#[test]
fn every_load_outcome_arm_round_trips() {
    assert_ron_round_trip(&EditorLoadOutcomeNet::Loaded {
        key: EditorKeyNet::new("flak_vest".to_owned()),
    });
    assert_ron_round_trip(&EditorLoadOutcomeNet::NoSuchKey {
        key:   EditorKeyNet::new("no_such_vest".to_owned()),
        known: vec![EditorKeyNet::new("flak_vest".to_owned())],
    });
    assert_ron_round_trip(&EditorLoadOutcomeNet::Refused(
        EditorRefusalNet::NoLoadAction,
    ));
}

#[test]
fn every_save_outcome_arm_round_trips() {
    assert_ron_round_trip(&EditorSaveOutcomeNet::Wrote {
        path: SavedPathNet::from_path(Path::new("assets/content/armor/flak_vest.armor.ron")),
    });
    assert_ron_round_trip(&EditorSaveOutcomeNet::Failed(
        EditorSaveFaultNet::from_fault(&EditorSaveFault::EmptyName),
    ));
    assert_ron_round_trip(&EditorSaveOutcomeNet::Refused(
        EditorRefusalNet::NameBelongsToPrefabOnly,
    ));
}

#[test]
fn a_missed_key_carries_both_the_asked_key_and_the_known_ones() {
    let outcome = EditorLoadOutcomeNet::NoSuchKey {
        key:   EditorKeyNet::new("no_such_vest".to_owned()),
        known: vec![EditorKeyNet::new("flak_vest".to_owned())],
    };
    let EditorLoadOutcomeNet::NoSuchKey { key, known } = outcome else {
        unreachable!("built as a NoSuchKey above");
    };
    assert_eq!(
        *key, "no_such_vest",
        "the miss names the key the client asked for, so it can tell which call missed",
    );
    assert!(
        known.iter().any(|entry| **entry == *"flak_vest"),
        "the miss lists the keys the registry does hold, so one round trip fixes the call: \
         {known:?}",
    );
}

#[test]
fn the_outcomes_trace_usable_shapes() {
    assert_schema_is_usable::<EditorNewOutcomeNet>("EditorNewOutcomeNet");
    assert_schema_is_usable::<EditorLoadOutcomeNet>("EditorLoadOutcomeNet");
    assert_schema_is_usable::<EditorSaveOutcomeNet>("EditorSaveOutcomeNet");
}
