use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    net_qa::wire::{EditorFieldNet, TerrainKindNet},
    terrain_form::TerrainKindChoice,
};

#[test]
fn every_field_arm_round_trips() {
    for choice in TerrainKindChoice::SEGMENT_ORDER {
        assert_ron_round_trip(&EditorFieldNet::Kind(TerrainKindNet::from_choice(choice)));
    }
}

#[test]
fn the_field_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorFieldNet>("EditorFieldNet");
}
