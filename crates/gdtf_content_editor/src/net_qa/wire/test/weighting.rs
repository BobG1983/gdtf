use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryName, InjuryWeight, InjuryWeighting, WeightedInjuryEntry},
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    DamageContextNet, EditorFieldNet, EditorListIndexNet, InjuryKeyNet, InjuryWeightNet,
    WeightingBucketNet, WeightingRowNet, WeightingTableNet,
};

fn an_entry(key: &str, weight: u32) -> WeightedInjuryEntry {
    WeightedInjuryEntry::new(InjuryName::new(key.to_owned()), InjuryWeight::new(weight))
}

fn a_row() -> WeightingRowNet {
    WeightingRowNet::from_entry(&an_entry("torn_hamstring", 8))
}

fn a_table() -> WeightingTableNet {
    WeightingTableNet::from_weighting(&InjuryWeighting {
        category: InjuryCategory::Leg,
        context:  DamageContext::Melee,
        minor:    vec![an_entry("torn_hamstring", 8), an_entry("twisted_ankle", 5)],
        major:    vec![an_entry("shattered_knee", 6)],
        critical: Vec::new(),
    })
}

#[test]
fn every_damage_context_round_trips() {
    for context in DamageContext::ALL {
        assert_ron_round_trip(&DamageContextNet::from_context(context));
    }
}

#[test]
fn every_bucket_round_trips() {
    for bucket in [
        WeightingBucketNet::Minor,
        WeightingBucketNet::Major,
        WeightingBucketNet::Critical,
    ] {
        assert_ron_round_trip(&bucket);
    }
}

#[test]
fn the_row_and_its_weight_round_trip() {
    assert_ron_round_trip(&InjuryWeightNet::from_weight(InjuryWeight::new(8)));
    assert_ron_round_trip(&a_row());
}

#[test]
fn the_whole_table_round_trips() {
    assert_ron_round_trip(&a_table());
}

#[test]
fn both_weighting_row_field_arms_round_trip() {
    for bucket in [
        WeightingBucketNet::Minor,
        WeightingBucketNet::Major,
        WeightingBucketNet::Critical,
    ] {
        assert_ron_round_trip(&EditorFieldNet::WeightingRowInjury {
            bucket,
            index: EditorListIndexNet::new(1),
            injury: InjuryKeyNet::new("twisted_ankle"),
        });
        assert_ron_round_trip(&EditorFieldNet::WeightingRowWeight {
            bucket,
            index: EditorListIndexNet::new(0),
            weight: InjuryWeightNet::from_weight(InjuryWeight::new(4)),
        });
    }
}

#[test]
fn every_damage_context_reads_back_as_the_sims_own() {
    for context in DamageContext::ALL {
        assert_eq!(
            DamageContextNet::from_context(context).to_context(),
            context
        );
    }
}

#[test]
fn a_weight_reads_back_as_the_sims_own() {
    let weight = InjuryWeight::new(12);
    assert_eq!(InjuryWeightNet::from_weight(weight).to_weight(), weight);
}

#[test]
fn the_weighting_types_trace_usable_shapes() {
    assert_schema_is_usable::<DamageContextNet>("DamageContextNet");
    assert_schema_is_usable::<WeightingBucketNet>("WeightingBucketNet");
    assert_schema_is_usable::<InjuryWeightNet>("InjuryWeightNet");
    assert_schema_is_usable::<WeightingRowNet>("WeightingRowNet");
    assert_schema_is_usable::<WeightingTableNet>("WeightingTableNet");
}
