use gdtf_battle_sim::injuries::InjuryWeight;

use super::super::assert_ron_round_trip;
use crate::net_qa::wire::{
    EditorFieldNet, EditorListIndexNet, InjuryKeyNet, InjuryWeightNet, WeightingBucketNet,
    WeightingFieldNet,
};

#[test]
fn both_weighting_row_field_arms_round_trip() {
    for bucket in [
        WeightingBucketNet::Minor,
        WeightingBucketNet::Major,
        WeightingBucketNet::Critical,
    ] {
        assert_ron_round_trip(&EditorFieldNet::Weighting(WeightingFieldNet::RowInjury {
            bucket,
            index: EditorListIndexNet::new(1),
            injury: InjuryKeyNet::new("twisted_ankle"),
        }));
        assert_ron_round_trip(&EditorFieldNet::Weighting(WeightingFieldNet::RowWeight {
            bucket,
            index: EditorListIndexNet::new(0),
            weight: InjuryWeightNet::from_weight(InjuryWeight::new(4)),
        }));
    }
}
