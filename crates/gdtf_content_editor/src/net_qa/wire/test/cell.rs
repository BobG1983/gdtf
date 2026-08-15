use gdtf_battle_sim::metric::{Cell, CellLevel, Level};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::cell::{
    EditorCellLevelNet, EditorCellXNet, EditorCellYNet, EditorLevelNet,
};

#[test]
fn a_painted_cell_round_trips() {
    let slot = CellLevel::new(Cell::new(3, -2), Level::new(1));
    assert_ron_round_trip(&EditorCellLevelNet::from_slot(slot));
}

#[test]
fn two_different_slots_read_differently() {
    let low = EditorCellLevelNet::from_slot(CellLevel::new(Cell::new(3, -2), Level::new(0)));
    let high = EditorCellLevelNet::from_slot(CellLevel::new(Cell::new(3, -2), Level::new(1)));
    assert_ne!(
        low, high,
        "the storey is part of the wire slot, so two cells one above the other are not the same \
         illegal cell to a client",
    );
}

#[test]
fn each_coordinate_round_trips_on_its_own() {
    assert_ron_round_trip(&EditorCellXNet::new(-4));
    assert_ron_round_trip(&EditorCellYNet::new(7));
    assert_ron_round_trip(&EditorLevelNet::new(2));
}

#[test]
fn the_painted_cell_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorCellXNet>("EditorCellXNet");
    assert_schema_is_usable::<EditorCellYNet>("EditorCellYNet");
    assert_schema_is_usable::<EditorLevelNet>("EditorLevelNet");
    assert_schema_is_usable::<EditorCellLevelNet>("EditorCellLevelNet");
}
