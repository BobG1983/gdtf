use std::path::Path;

use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    mcp::wire::{
        EditorContentNameNet, EditorGridSizeNet, EditorKeyNet, EditorLoadOutcomeNet,
        EditorLoadPrefabOutcomeNet, EditorNewOutcomeNet, EditorRefusalNet, EditorSaveFaultNet,
        EditorSaveOutcomeNet, PrefabPlacementCountNet, SavedPathNet, SpawnRoleNet, ThemeKeyNet,
    },
    save_record::EditorSaveFault,
};

// A UUID's own hyphenated text, the form a theme key crosses the wire in.
const A_THEME: &str = "00000000-0000-0000-0000-01840a900001";

// `EditorContentNameNet` declares no constructor, so a case builds one by deserializing.
fn content_name(wire: &str) -> EditorContentNameNet {
    match ron::de::from_str::<EditorContentNameNet>(wire) {
        Ok(name) => name,
        Err(fault) => unreachable!("`{wire}` is the wire form of a content name: {fault}"),
    }
}

fn a_grid() -> EditorGridSizeNet {
    match GridSize::new(GridWidth::new(12), GridHeight::new(12), GridLevels::new(1)) {
        Ok(size) => EditorGridSizeNet::from_size(size),
        Err(fault) => unreachable!("12x12x1 sits inside every grid bound: {fault}"),
    }
}

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
fn every_prefab_load_outcome_arm_round_trips() {
    assert_ron_round_trip(&EditorLoadPrefabOutcomeNet::Opened {
        grid_size:  a_grid(),
        placements: PrefabPlacementCountNet::new(4),
    });
    assert_ron_round_trip(&EditorLoadPrefabOutcomeNet::NoSuchPrefab {
        name:  content_name("\"no_such_room\""),
        theme: ThemeKeyNet::new(A_THEME.to_owned()),
        size:  a_grid(),
        role:  SpawnRoleNet::Player,
    });
}

#[test]
fn a_missed_prefab_carries_the_whole_key_that_was_asked_for() {
    let outcome = EditorLoadPrefabOutcomeNet::NoSuchPrefab {
        name:  content_name("\"no_such_room\""),
        theme: ThemeKeyNet::new(A_THEME.to_owned()),
        size:  a_grid(),
        role:  SpawnRoleNet::Player,
    };
    let EditorLoadPrefabOutcomeNet::NoSuchPrefab {
        name,
        theme,
        size,
        role,
    } = outcome
    else {
        unreachable!("built as a NoSuchPrefab above");
    };
    assert_eq!(
        (&*name, &*theme, size, role),
        (
            &"no_such_room".to_owned(),
            &A_THEME.to_owned(),
            a_grid(),
            SpawnRoleNet::Player
        ),
        "a prefab is found by its name AND its three-part key, so the miss names all four or a \
         client cannot tell which part it got wrong",
    );
}

#[test]
fn the_outcomes_trace_usable_shapes() {
    assert_schema_is_usable::<EditorNewOutcomeNet>("EditorNewOutcomeNet");
    assert_schema_is_usable::<EditorLoadOutcomeNet>("EditorLoadOutcomeNet");
    assert_schema_is_usable::<EditorLoadPrefabOutcomeNet>("EditorLoadPrefabOutcomeNet");
    assert_schema_is_usable::<EditorSaveOutcomeNet>("EditorSaveOutcomeNet");
}
