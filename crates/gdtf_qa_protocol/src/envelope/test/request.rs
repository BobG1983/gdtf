//! Exhaustive per-variant round-trip + parity-forcing pins for [`QaRequest`]
//! (GTW-734).

use crate::{
    envelope::{ProtocolVersion, QaRequest},
    ids::{EventCap, SeedNet, ShotName, SituationRef},
    intent::NetIntent,
    test_support::assert_ron_round_trip,
};

/// Every [`QaRequest`] variant — the round-trip table, kept in lock-step with the enum
/// by [`qa_request_is_exhaustive`].
fn qa_request_cases() -> Vec<QaRequest> {
    vec![
        QaRequest::Hello(ProtocolVersion::new(1)),
        QaRequest::GetAppFlow,
        QaRequest::GetBattleState,
        QaRequest::Inject(NetIntent::Reload),
        QaRequest::TakeScreenshot {
            name: Some(ShotName::new("aim_check".to_owned())),
        },
        QaRequest::GetOutput {
            max: Some(EventCap::new(16)),
        },
        QaRequest::StartBattle {
            situation: SituationRef::new("skirmish".to_owned()),
            seed:      Some(SeedNet::new(7)),
        },
    ]
}

/// The wildcard-free witness — adding a [`QaRequest`] variant breaks this `match` until
/// it (and [`qa_request_cases`]) gain the new arm.
fn qa_request_is_exhaustive(request: &QaRequest) {
    match request {
        QaRequest::Hello(_)
        | QaRequest::GetAppFlow
        | QaRequest::GetBattleState
        | QaRequest::Inject(_)
        | QaRequest::TakeScreenshot { .. }
        | QaRequest::GetOutput { .. }
        | QaRequest::StartBattle { .. } => {}
    }
}

/// Every [`QaRequest`] variant round-trips through compact RON identically.
#[test]
fn qa_request_round_trips_every_variant() {
    let cases = qa_request_cases();
    assert_eq!(
        cases.len(),
        7,
        "the case table lists every QaRequest variant"
    );
    for case in &cases {
        qa_request_is_exhaustive(case);
        assert_ron_round_trip(case);
    }
}

/// The `Option`-carrying requests round-trip in their `None` form too.
#[test]
fn optional_request_fields_round_trip_when_absent() {
    assert_ron_round_trip(&QaRequest::TakeScreenshot { name: None });
    assert_ron_round_trip(&QaRequest::GetOutput { max: None });
    assert_ron_round_trip(&QaRequest::StartBattle {
        situation: SituationRef::new("ambush".to_owned()),
        seed:      None,
    });
}
