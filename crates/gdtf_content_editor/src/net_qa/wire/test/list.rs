use gdtf_battle_sim::terrain::facing::TerrainFacing;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{EditorListMemberNet, EditorListNet, EditorListOpNet, TerrainFacingNet};

#[test]
fn the_named_list_round_trips() {
    assert_ron_round_trip(&EditorListNet::EntrySides);
}

#[test]
fn every_toggle_arm_round_trips() {
    for facing in TerrainFacing::ALL {
        assert_ron_round_trip(&EditorListOpNet::Toggle(TerrainFacingNet::from_facing(
            facing,
        )));
    }
}

#[test]
fn every_member_round_trips() {
    for facing in TerrainFacing::ALL {
        assert_ron_round_trip(&EditorListMemberNet::from_facing(
            TerrainFacingNet::from_facing(facing),
        ));
    }
}

#[test]
fn the_list_types_trace_usable_shapes() {
    assert_schema_is_usable::<EditorListNet>("EditorListNet");
    assert_schema_is_usable::<EditorListOpNet>("EditorListOpNet");
    assert_schema_is_usable::<EditorListMemberNet>("EditorListMemberNet");
}
