//! Exhaustive per-variant round-trip + parity-forcing pins for [`QaRequest`]
//! (GTW-734).

use crate::{
    envelope::{FocusCommandNet, FocusStepNet, ProtocolVersion, QaRequest, StepperCommandNet},
    ids::{EventCap, FocusTargetNet, FrameDelay, SeedNet, ShotName, SituationRef},
    intent::NetIntent,
    test_support::assert_ron_round_trip,
    view::{EditorQueryKind, RequestKindNet},
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
        QaRequest::FocusControl(FocusCommandNet::Step(FocusStepNet::Next)),
        QaRequest::GetEditorQueryOptions,
        QaRequest::QueryEditor(EditorQueryKind::Draft),
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
        | QaRequest::ActivateMenuItem(_)
        | QaRequest::FocusControl(_)
        | QaRequest::GetEditorQueryOptions
        | QaRequest::QueryEditor(_) => {}
    }
}

/// Every [`QaRequest`] variant round-trips through compact RON identically.
#[test]
fn qa_request_round_trips_every_variant() {
    let cases = qa_request_cases();
    assert_eq!(
        cases.len(),
        13,
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

/// Every [`FocusCommandNet`] variant rides a `FocusControl` request identically — the
/// four step directions, a bare focus point, a bare activate, and an activate-by-token
/// (GTW-802). The wildcard-free witness forces a new command variant in.
#[test]
fn focus_control_round_trips_every_command() {
    let steps = [
        FocusStepNet::Next,
        FocusStepNet::Prev,
        FocusStepNet::Left,
        FocusStepNet::Right,
    ];
    let mut commands: Vec<FocusCommandNet> = steps.into_iter().map(FocusCommandNet::Step).collect();
    commands.push(FocusCommandNet::Focus(FocusTargetNet::new(7)));
    commands.push(FocusCommandNet::Activate);
    commands.push(FocusCommandNet::ActivateTarget(FocusTargetNet::new(8)));
    for command in commands {
        match command {
            FocusCommandNet::Step(step) => match step {
                FocusStepNet::Next
                | FocusStepNet::Prev
                | FocusStepNet::Left
                | FocusStepNet::Right => {}
            },
            FocusCommandNet::Focus(_)
            | FocusCommandNet::Activate
            | FocusCommandNet::ActivateTarget(_) => {}
        }
        assert_ron_round_trip(&QaRequest::FocusControl(command));
    }
}

/// Every [`EditorQueryKind`] rides a `QueryEditor` request identically, and
/// [`EditorQueryKind::ALL`] lists every one of them (GTW-805). The wildcard-free witness
/// forces a new topic into both.
#[test]
fn query_editor_round_trips_every_topic() {
    assert_eq!(
        EditorQueryKind::ALL.len(),
        5,
        "EditorQueryKind::ALL lists every topic",
    );
    for kind in EditorQueryKind::ALL {
        match kind {
            EditorQueryKind::Readiness
            | EditorQueryKind::Mode
            | EditorQueryKind::Session
            | EditorQueryKind::Draft
            | EditorQueryKind::Validation => {}
        }
        assert_ron_round_trip(&QaRequest::QueryEditor(kind));
    }
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
