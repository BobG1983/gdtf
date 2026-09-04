use gdtf_battle_sim::metric::{Cell, CellLevel, Level};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    mcp::wire::{EditorSaveFaultNet, save_fault::SaveFaultDetailNet},
    save_record::{EditorSaveFault, SaveFaultMessage},
};

fn every_fault() -> Vec<EditorSaveFault> {
    vec![
        EditorSaveFault::EmptyName,
        EditorSaveFault::MissingMountedWeapon,
        EditorSaveFault::NoTerrain,
        EditorSaveFault::DefaultFloorNotInTerrain,
        EditorSaveFault::IllegalCell(CellLevel::new(Cell::new(1, 2), Level::new(3))),
        EditorSaveFault::NoWorkspaceRoot,
        EditorSaveFault::Serialize(SaveFaultMessage::new("bad float".to_owned())),
        EditorSaveFault::Write(SaveFaultMessage::new("permission denied".to_owned())),
    ]
}

#[test]
fn every_save_fault_arm_round_trips() {
    for fault in every_fault() {
        assert_ron_round_trip(&EditorSaveFaultNet::from_fault(&fault));
    }
}

#[test]
fn every_save_fault_mirrors_to_its_own_name() {
    let mut seen: Vec<String> = Vec::new();
    for fault in every_fault() {
        let mirrored = EditorSaveFaultNet::from_fault(&fault);
        let named = format!("{mirrored:?}");
        let source = format!("{fault:?}");
        assert_eq!(
            named.split('(').next(),
            source.split('(').next(),
            "the wire mirror of {source} must carry that fault's own name",
        );
        assert!(
            !seen.contains(&named),
            "{named} is claimed by another fault — two faults that read the same on the wire are \
             indistinguishable to a client",
        );
        seen.push(named);
    }
}

#[test]
fn a_write_fault_carries_the_message_its_source_reported() {
    let fault = EditorSaveFault::Write(SaveFaultMessage::new("permission denied".to_owned()));
    let EditorSaveFaultNet::Write(detail) = EditorSaveFaultNet::from_fault(&fault) else {
        unreachable!("a Write fault mirrors to a Write fault: {fault:?}");
    };
    assert_eq!(
        *detail, "permission denied",
        "the wire fault names what the filesystem said, so a client need not read the log to \
         learn why the save failed",
    );
}

#[test]
fn a_fault_detail_round_trips_on_its_own() {
    assert_ron_round_trip(&SaveFaultDetailNet::new("permission denied".to_owned()));
}

#[test]
fn the_save_fault_types_trace_usable_shapes() {
    assert_schema_is_usable::<SaveFaultDetailNet>("SaveFaultDetailNet");
    assert_schema_is_usable::<EditorSaveFaultNet>("EditorSaveFaultNet");
}
