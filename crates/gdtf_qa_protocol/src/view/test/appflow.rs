//! Round-trip pins for the app-flow snapshot and its enum witnesses (GTW-734, GTW-746).

use crate::{
    test_support::assert_ron_round_trip,
    view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, RequestKindNet},
};

/// The app-flow snapshot (with a populated affordance list) round-trips; the empty form
/// round-trips too.
#[test]
fn app_flow_round_trips() {
    assert_ron_round_trip(&AppFlowView::new(
        AppStateNet::Running,
        BattleActiveNet::new(true),
        RequestKindNet::ALL.to_vec(),
        CaughtUpNet::new(true),
        None,
        None,
    ));
    assert_ron_round_trip(&AppFlowView::new(
        AppStateNet::Intro,
        BattleActiveNet::new(false),
        Vec::new(),
        CaughtUpNet::new(false),
        None,
        None,
    ));
}

/// Every [`RequestKindNet`] round-trips, and the witness forces new variants in — kept in
/// lock-step with the [`RequestKindNet::ALL`] table the server filters.
#[test]
fn request_kind_round_trips_every_variant() {
    assert_eq!(
        RequestKindNet::ALL.len(),
        13,
        "RequestKindNet::ALL lists every request kind"
    );
    for kind in RequestKindNet::ALL {
        match kind {
            RequestKindNet::Hello
            | RequestKindNet::GetAppFlow
            | RequestKindNet::GetBattleState
            | RequestKindNet::Inject
            | RequestKindNet::TakeScreenshot
            | RequestKindNet::ScreenshotAfter
            | RequestKindNet::GetOutput
            | RequestKindNet::StartBattle
            | RequestKindNet::StepperControl
            | RequestKindNet::ActivateMenuItem
            | RequestKindNet::FocusControl
            | RequestKindNet::GetEditorQueryOptions
            | RequestKindNet::QueryEditor => {}
        }
        assert_ron_round_trip(&kind);
    }
}

/// Every [`AppStateNet`] round-trips; the witness forces new variants in.
#[test]
fn app_state_round_trips_every_variant() {
    for state in [
        AppStateNet::Init,
        AppStateNet::Load,
        AppStateNet::Intro,
        AppStateNet::Running,
        AppStateNet::Teardown,
    ] {
        match state {
            AppStateNet::Init
            | AppStateNet::Load
            | AppStateNet::Intro
            | AppStateNet::Running
            | AppStateNet::Teardown => {}
        }
        assert_ron_round_trip(&state);
    }
}
