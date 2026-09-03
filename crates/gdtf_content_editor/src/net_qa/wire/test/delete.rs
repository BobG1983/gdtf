use gdtf_assets::{ContentMemberKey, FindingFamily, ReferenceField, ReferringRecord};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::{
    delete::{DeleteOutcome, DeleteRefusal},
    net_qa::wire::DeleteOutcomeNet,
};

// One referring record, the shape the in-use check answers with.
fn a_referrer() -> ReferringRecord {
    ReferringRecord::new(
        FindingFamily::new("GangRegistry".to_owned()),
        ContentMemberKey::new("fixture_gang".to_owned()),
        ReferenceField::new("members[].weapon".to_owned()),
    )
}

#[test]
fn every_delete_outcome_arm_round_trips() {
    assert_ron_round_trip(&DeleteOutcomeNet::from_outcome(&DeleteOutcome::Removed));
    assert_ron_round_trip(&DeleteOutcomeNet::from_outcome(&DeleteOutcome::Refused(
        DeleteRefusal::NoEntry,
    )));
    assert_ron_round_trip(&DeleteOutcomeNet::from_outcome(&DeleteOutcome::Refused(
        DeleteRefusal::NoRecord,
    )));
    assert_ron_round_trip(&DeleteOutcomeNet::from_outcome(&DeleteOutcome::Refused(
        DeleteRefusal::InUse(vec![a_referrer()]),
    )));
}

#[test]
fn an_in_use_refusal_carries_every_referring_record_the_check_answered_with() {
    let outcome =
        DeleteOutcomeNet::from_outcome(&DeleteOutcome::Refused(DeleteRefusal::InUse(vec![
            a_referrer(),
        ])));
    let encoded = ron::ser::to_string(&outcome);
    assert!(
        encoded.is_ok(),
        "a delete outcome serializes to compact RON: {outcome:?}",
    );
    let Ok(encoded) = encoded else { return };
    assert!(
        encoded.contains("fixture_gang") && encoded.contains("members[].weapon"),
        "the refusal must name the referring record's key AND the field it holds the \
         reference in, or a client cannot fix the reference; got `{encoded}`",
    );
}

#[test]
fn the_delete_outcome_traces_a_usable_shape() {
    assert_schema_is_usable::<DeleteOutcomeNet>("DeleteOutcomeNet");
}
