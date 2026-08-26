use super::super::assert_ron_round_trip;
use crate::{
    net_qa::wire::{EditorFieldNet, TerrainKindNet},
    terrain_form::TerrainKindChoice,
};

#[test]
fn every_terrain_field_arm_round_trips() {
    for choice in TerrainKindChoice::SEGMENT_ORDER {
        assert_ron_round_trip(&EditorFieldNet::TerrainKind(TerrainKindNet::from_choice(
            choice,
        )));
    }
}
