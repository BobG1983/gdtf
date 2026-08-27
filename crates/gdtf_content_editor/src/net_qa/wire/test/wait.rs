use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{ContentFamilyNet, EditorWaitConditionNet};

#[test]
fn the_checks_complete_condition_round_trips() {
    assert_ron_round_trip(&EditorWaitConditionNet::ChecksComplete);
}

#[test]
fn a_registry_rearmed_condition_round_trips_for_every_family() {
    for family in ContentFamilyNet::ALL {
        assert_ron_round_trip(&EditorWaitConditionNet::RegistryRearmed { family });
    }
}

#[test]
fn a_rearm_condition_keeps_the_family_it_named_across_the_wire() {
    let sent = EditorWaitConditionNet::RegistryRearmed {
        family: ContentFamilyNet::Sprite,
    };
    let Ok(encoded) = ron::ser::to_string(&sent) else {
        unreachable!("a wire value serializes to compact RON: {sent:?}");
    };
    let Ok(decoded) = ron::de::from_str::<EditorWaitConditionNet>(&encoded) else {
        unreachable!("the compact RON `{encoded}` parses back to its type");
    };
    assert_eq!(
        decoded,
        EditorWaitConditionNet::RegistryRearmed {
            family: ContentFamilyNet::Sprite,
        },
        "a wait that names one family must not come back naming another, or it would park on \
         the wrong registry: `{encoded}`",
    );
}

#[test]
fn the_wait_condition_traces_a_usable_shape() {
    assert_schema_is_usable::<EditorWaitConditionNet>("EditorWaitConditionNet");
}
