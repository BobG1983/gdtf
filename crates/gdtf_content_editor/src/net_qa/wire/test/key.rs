use std::path::Path;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{EditorContentNameNet, EditorKeyNet, SavedPathNet};

fn content_name(wire: &str) -> EditorContentNameNet {
    match ron::de::from_str::<EditorContentNameNet>(wire) {
        Ok(name) => name,
        Err(fault) => unreachable!("`{wire}` is the wire form of a content name: {fault}"),
    }
}

#[test]
fn a_registry_key_round_trips() {
    assert_ron_round_trip(&EditorKeyNet::new("ash_optic".to_owned()));
}

#[test]
fn a_content_name_round_trips() {
    assert_ron_round_trip(&content_name("\"east corridor\""));
}

#[test]
fn a_written_path_round_trips() {
    assert_ron_round_trip(&SavedPathNet::from_path(Path::new(
        "assets/content/armor/flak_vest.armor.ron",
    )));
}

#[test]
fn a_written_path_carries_the_path_it_mirrored() {
    let path = Path::new("assets/content/armor/flak_vest.armor.ron");
    assert_eq!(
        *SavedPathNet::from_path(path),
        path.display().to_string(),
        "the wire path is the writer's own path, so a client can compare it to a file it reads",
    );
}

#[test]
fn the_key_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorKeyNet>("EditorKeyNet");
    assert_schema_is_usable::<EditorContentNameNet>("EditorContentNameNet");
    assert_schema_is_usable::<SavedPathNet>("SavedPathNet");
}
