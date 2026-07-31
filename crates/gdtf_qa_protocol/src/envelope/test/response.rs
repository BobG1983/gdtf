//! Exhaustive per-variant round-trip + parity-forcing pins for [`QaResponse`]
//! (GTW-734).

use crate::{
    command::{
        ArgSchemaJson, CommandAvailability, CommandCatalogue, CommandEntry, CommandName,
        CommandOutcome, CommandReplyJson, CommandSummary, ReplySchemaJson,
    },
    envelope::{
        FocusControlReceipt, HelloFacts, InjectReceipt, MenuActivationReceipt, ProtocolVersion,
        QaError, QaResponse, RejectReason, ScreenshotAfterResult, ScreenshotResult, ServerNameNet,
        StepperReceipt,
    },
    events::{DroppedCount, EventBatch},
    test_support::assert_ron_round_trip,
    view::{
        AppFlowView, AppStateNet, BattleActiveNet, BattleView, CaughtUpNet, EditorQueryKind,
        EditorQueryOptionsView, EditorQueryReply, EditorQueryTopicView, EditorQueryView,
        EditorReadinessNet, ExploredCellCountNet, FactionNet, FogView, GridHeightNet,
        GridLevelsNet, GridSizeNet, GridWidthNet, RequestKindNet, SelectionView,
        TerrainSummaryView, TurnView, VisibleCellCountNet,
    },
};

/// A minimal (empty-field) battle snapshot — enough to exercise the `Battle` reply's
/// round-trip without rebuilding the full ganger fixture (that lives in the `view`
/// suite).
fn an_empty_battle() -> BattleView {
    BattleView::new(
        vec![],
        TerrainSummaryView::new(
            GridSizeNet::new(
                GridWidthNet::new(1),
                GridHeightNet::new(1),
                GridLevelsNet::new(1),
            ),
            vec![],
            vec![],
        ),
        vec![],
        FogView::new(VisibleCellCountNet::new(0), ExploredCellCountNet::new(0)),
        SelectionView::new(None),
        TurnView::new(FactionNet::new(0), FactionNet::new(0)),
    )
}

/// Every [`QaResponse`] variant — the round-trip table, kept in lock-step with the enum
/// by [`qa_response_is_exhaustive`].
fn qa_response_cases() -> Vec<QaResponse> {
    vec![
        QaResponse::HelloOk(HelloFacts::new(
            ProtocolVersion::new(1),
            ServerNameNet::new("gdtf".to_owned()),
        )),
        QaResponse::AppFlow(AppFlowView::new(
            AppStateNet::Running,
            BattleActiveNet::new(false),
            RequestKindNet::ALL.to_vec(),
            CaughtUpNet::new(true),
            None,
            None,
        )),
        QaResponse::Battle(an_empty_battle()),
        QaResponse::Injected(InjectReceipt::Queued),
        QaResponse::Screenshot(ScreenshotResult::TimedOut),
        QaResponse::ScreenshotAfter(ScreenshotAfterResult::TimedOut),
        QaResponse::Output(EventBatch::new(vec![], DroppedCount::new(0))),
        QaResponse::StepperControlled(StepperReceipt::Latched),
        QaResponse::MenuItemActivated(MenuActivationReceipt::Activated),
        QaResponse::FocusControlled(FocusControlReceipt::Applied),
        QaResponse::EditorQueryOptions(EditorQueryOptionsView::new(
            EditorReadinessNet::Load,
            vec![EditorQueryTopicView::offered(EditorQueryKind::Readiness)],
        )),
        QaResponse::EditorQuery(EditorQueryReply::new(
            EditorReadinessNet::Editing,
            EditorQueryView::Readiness(EditorReadinessNet::Editing),
        )),
        QaResponse::Catalogue(CommandCatalogue::new(
            ServerNameNet::new("gdtf-net-qa".to_owned()),
            vec![CommandEntry::new(
                CommandName::from_static("app.phase"),
                CommandSummary::from_static("Read the whole state tuple."),
                ArgSchemaJson::new(r#"{"type":"object"}"#.to_owned()),
                ReplySchemaJson::new(r#"{"type":"object"}"#.to_owned()),
                CommandAvailability::Available,
            )],
        )),
        QaResponse::Outcome(CommandOutcome::Ran {
            reply:       CommandReplyJson::new(r#"{"app":"Running"}"#.to_owned()),
            attachments: vec![],
        }),
        QaResponse::Error(QaError::Busy),
    ]
}

/// The wildcard-free witness — adding a [`QaResponse`] variant breaks this `match`
/// until it (and [`qa_response_cases`]) gain the new arm.
fn qa_response_is_exhaustive(response: &QaResponse) {
    match response {
        QaResponse::HelloOk(_)
        | QaResponse::AppFlow(_)
        | QaResponse::Battle(_)
        | QaResponse::Injected(_)
        | QaResponse::Screenshot(_)
        | QaResponse::ScreenshotAfter(_)
        | QaResponse::Output(_)
        | QaResponse::StepperControlled(_)
        | QaResponse::MenuItemActivated(_)
        | QaResponse::FocusControlled(_)
        | QaResponse::EditorQueryOptions(_)
        | QaResponse::EditorQuery(_)
        | QaResponse::Catalogue(_)
        | QaResponse::Outcome(_)
        | QaResponse::Error(_) => {}
    }
}

/// Every [`QaResponse`] variant round-trips through compact RON identically.
#[test]
fn qa_response_round_trips_every_variant() {
    let cases = qa_response_cases();
    assert_eq!(
        cases.len(),
        15,
        "the case table lists every QaResponse variant"
    );
    for case in &cases {
        qa_response_is_exhaustive(case);
        assert_ron_round_trip(case);
    }
}

/// The menu-item activation receipt round-trips in both forms — the dispatched
/// activation and the typed stale-token rejection (GTW-787).
#[test]
fn menu_item_activated_round_trips_both_forms() {
    assert_ron_round_trip(&QaResponse::MenuItemActivated(
        MenuActivationReceipt::Activated,
    ));
    assert_ron_round_trip(&QaResponse::MenuItemActivated(
        MenuActivationReceipt::Rejected(RejectReason::StaleToken),
    ));
}

/// The focus-control receipt round-trips in both forms — the dispatched command and the
/// typed stale-token rejection (GTW-802).
#[test]
fn focus_controlled_round_trips_both_forms() {
    assert_ron_round_trip(&QaResponse::FocusControlled(FocusControlReceipt::Applied));
    assert_ron_round_trip(&QaResponse::FocusControlled(FocusControlReceipt::Rejected(
        RejectReason::StaleToken,
    )));
}
