use gdtf_battle_sim::injuries::StatTarget;

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::StatTargetNet;

#[test]
fn every_stat_target_round_trips() {
    for target in StatTarget::ALL {
        assert_ron_round_trip(&StatTargetNet::from_target(target));
    }
}

#[test]
fn the_stat_target_traces_a_usable_shape() {
    assert_schema_is_usable::<StatTargetNet>("StatTargetNet");
}
