use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::TileRoleNet;

#[test]
fn every_tile_role_round_trips() {
    for role in TileRoleNet::ALL {
        assert_ron_round_trip(&role);
    }
}

#[test]
fn the_tile_role_traces_a_usable_shape() {
    assert_schema_is_usable::<TileRoleNet>("TileRoleNet");
}
