use gdtf_assets::{ContentFinding, FindingDetail, FindingReferrer};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{ChecksCompleteNet, ValidationFindingNet, ValidationPublishedNet};

fn degraded_fallback() -> ContentFinding {
    ContentFinding::DegradedFallback {
        context: FindingReferrer::new("theme `ash_wastes`".to_owned()),
        detail:  FindingDetail::new("no default floor authored".to_owned()),
    }
}

#[test]
fn both_pass_markers_round_trip() {
    for complete in [true, false] {
        assert_ron_round_trip(&ChecksCompleteNet::new(complete));
    }
    for published in [true, false] {
        assert_ron_round_trip(&ValidationPublishedNet::new(published));
    }
}

#[test]
fn a_rendered_finding_round_trips() {
    assert_ron_round_trip(&ValidationFindingNet::from_finding(&degraded_fallback()));
}

#[test]
fn a_finding_is_rendered_through_its_own_display() {
    let finding = degraded_fallback();
    assert_eq!(
        *ValidationFindingNet::from_finding(&finding),
        finding.to_string(),
        "the wire line is the finding's own `Display`, so a client reads what the editor's log \
         printed rather than a second rendering that can drift from it",
    );
}

#[test]
fn the_validation_types_trace_usable_shapes() {
    assert_schema_is_usable::<ChecksCompleteNet>("ChecksCompleteNet");
    assert_schema_is_usable::<ValidationPublishedNet>("ValidationPublishedNet");
    assert_schema_is_usable::<ValidationFindingNet>("ValidationFindingNet");
}
