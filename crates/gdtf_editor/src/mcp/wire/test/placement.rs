use gdtf_battle_sim::metric::{Cell, CellLevel, Level};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    mcp::wire::placement::{IllegalReasonNet, PlacementVerdictNet},
    placement::{IllegalReason, PlacementVerdict},
};

// Both ways a placement can be turned down, so neither arm can be dropped unnoticed.
const BOTH_REASONS: [IllegalReason; 2] =
    [IllegalReason::OutOfBounds, IllegalReason::SlabSealsLadder];

#[test]
fn every_illegal_reason_mirrors_to_its_own_name() {
    let mut seen: Vec<String> = Vec::with_capacity(BOTH_REASONS.len());
    for reason in BOTH_REASONS {
        let mirrored = format!("{:?}", IllegalReasonNet::from_reason(reason));
        assert_eq!(
            mirrored,
            format!("{reason:?}"),
            "the wire mirror of {reason:?} must carry that reason's own name",
        );
        assert!(
            !seen.contains(&mirrored),
            "{reason:?} maps onto {mirrored}, which another reason already claims. Two \
             refusals that read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_illegal_reason_round_trips() {
    for reason in BOTH_REASONS {
        assert_ron_round_trip(&IllegalReasonNet::from_reason(reason));
    }
}

#[test]
fn a_legal_verdict_carries_the_slot_it_clears() {
    let above = CellLevel::new(Cell::new(4, -1), Level::new(2));
    let cleared = PlacementVerdictNet::from_verdict(&PlacementVerdict::legal_clearing(above));
    assert_ne!(
        cleared,
        PlacementVerdictNet::from_verdict(&PlacementVerdict::legal()),
        "a placement that clears a slab above is not the same answer as one that clears \
         nothing, and a client that cannot tell them apart would not know a slot was emptied",
    );
    assert_ron_round_trip(&cleared);
}

#[test]
fn every_verdict_arm_round_trips() {
    assert_ron_round_trip(&PlacementVerdictNet::from_verdict(
        &PlacementVerdict::legal(),
    ));
    for reason in BOTH_REASONS {
        assert_ron_round_trip(&PlacementVerdictNet::from_verdict(
            &PlacementVerdict::Illegal(reason),
        ));
    }
}

#[test]
fn the_placement_types_trace_usable_shapes() {
    assert_schema_is_usable::<IllegalReasonNet>("IllegalReasonNet");
    assert_schema_is_usable::<PlacementVerdictNet>("PlacementVerdictNet");
}
