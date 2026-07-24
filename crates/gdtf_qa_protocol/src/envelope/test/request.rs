//! Exhaustive per-variant round-trip + parity-forcing pins for [`QaRequest`]
//! (GTW-734).

use crate::{
    envelope::{ProtocolVersion, QaRequest, StepperCommandNet},
    ids::{EventCap, FocusTargetNet, FrameDelay, SeedNet, ShotName, SituationRef},
    intent::NetIntent,
    test_support::assert_ron_round_trip,
    view::RequestKindNet,
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
        QaRequest::ScreenshotAfter {
            intent:      NetIntent::Reload,
            frame_delay: FrameDelay::new(15),
            name:        Some(ShotName::new("post_reload".to_owned())),
        },
        QaRequest::GetOutput {
            max: Some(EventCap::new(16)),
        },
        QaRequest::StartBattle {
            situation: SituationRef::new("skirmish".to_owned()),
            seed:      Some(SeedNet::new(7)),
        },
        QaRequest::StepperControl(StepperCommandNet::Next),
        QaRequest::ActivateMenuItem(FocusTargetNet::new(42)),
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
        | QaRequest::ScreenshotAfter { .. }
        | QaRequest::GetOutput { .. }
        | QaRequest::StartBattle { .. }
        | QaRequest::StepperControl(_)
        | QaRequest::ActivateMenuItem(_) => {}
    }
}

/// Every [`QaRequest`] variant round-trips through compact RON identically.
#[test]
fn qa_request_round_trips_every_variant() {
    let cases = qa_request_cases();
    assert_eq!(
        cases.len(),
        10,
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
    assert_ron_round_trip(&QaRequest::ScreenshotAfter {
        intent:      NetIntent::EndTurn,
        frame_delay: FrameDelay::new(0),
        name:        None,
    });
    assert_ron_round_trip(&QaRequest::GetOutput { max: None });
    assert_ron_round_trip(&QaRequest::StartBattle {
        situation: SituationRef::new("ambush".to_owned()),
        seed:      None,
    });
}

/// Every [`QaRequest`] variant maps to a DISTINCT [`RequestKindNet`], and a full case
/// table maps onto exactly [`RequestKindNet::ALL`] — so the request vocabulary and the
/// affordance vocabulary cannot drift apart. Adding a `QaRequest` variant without a
/// matching kind breaks [`QaRequest::kind`]'s wildcard-free `match`, which this exercises.
#[test]
fn every_request_maps_to_a_distinct_kind() {
    let kinds: Vec<RequestKindNet> = qa_request_cases().iter().map(QaRequest::kind).collect();
    // One kind per case, and every kind distinct.
    assert_eq!(kinds.len(), RequestKindNet::ALL.len());
    for (index, first) in kinds.iter().enumerate() {
        for second in &kinds[index + 1..] {
            assert_ne!(
                first, second,
                "each QaRequest variant must map to a distinct RequestKindNet",
            );
        }
    }
    // The case table's kinds cover exactly RequestKindNet::ALL.
    for kind in RequestKindNet::ALL {
        assert!(
            kinds.contains(&kind),
            "the QaRequest case table must cover {kind:?}",
        );
    }
}
