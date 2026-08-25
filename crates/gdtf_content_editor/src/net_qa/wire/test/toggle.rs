use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::TerrainToggleNet;

// Both directions a tick can go, so neither arm can be dropped unnoticed.
const BOTH_WAYS: [TerrainToggleNet; 2] = [TerrainToggleNet::Added, TerrainToggleNet::Removed];

#[test]
fn both_toggle_directions_round_trip() {
    let directions: [TerrainToggleNet; 2] = BOTH_WAYS;
    for direction in directions {
        assert_ron_round_trip(&direction);
    }
}

#[test]
fn the_terrain_toggle_traces_a_usable_shape() {
    assert_schema_is_usable::<TerrainToggleNet>("TerrainToggleNet");
}
