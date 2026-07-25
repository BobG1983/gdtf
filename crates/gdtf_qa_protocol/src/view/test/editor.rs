//! Round-trip pins for the editor query view family + exhaustive witnesses for its enums
//! (GTW-805).

use crate::{
    test_support::assert_ron_round_trip,
    view::{
        EditorDraftFieldNameNet, EditorDraftFieldValueNet, EditorDraftFieldView, EditorDraftView,
        EditorFindingDetailNet, EditorFindingKindNet, EditorFindingSubjectNet, EditorFindingView,
        EditorModeLabelNet, EditorModeNet, EditorModeView, EditorQueryKind, EditorQueryOptionsView,
        EditorQueryReply, EditorQueryTopicView, EditorQueryView, EditorReadinessNet,
        EditorSessionView, EditorTabIndexNet, EditorTerrainKeyNet, EditorThemeKeyNet,
        EditorValidationView, GridHeightNet, GridLevelsNet, GridSizeNet, GridWidthNet,
        ValidationChecksCompleteNet,
    },
};

/// A representative mode snapshot — the PREFAB tab, third in the tab order.
fn a_mode_view() -> EditorModeView {
    EditorModeView::new(
        EditorModeNet::Prefab,
        EditorModeLabelNet::new("PREFAB".to_owned()),
        EditorTabIndexNet::new(2),
    )
}

/// A representative session snapshot — a chosen theme with a resolved floor and a paint
/// tile.
fn a_session_view() -> EditorSessionView {
    EditorSessionView::new(
        EditorThemeKeyNet::new("6e5e9a3c-0000-4000-8000-000000000001".to_owned()),
        Some(EditorTerrainKeyNet::new(
            "6e5e9a3c-0000-4000-8000-000000000002".to_owned(),
        )),
        GridSizeNet::new(
            GridWidthNet::new(24),
            GridHeightNet::new(18),
            GridLevelsNet::new(3),
        ),
        Some(EditorTerrainKeyNet::new(
            "6e5e9a3c-0000-4000-8000-000000000003".to_owned(),
        )),
    )
}

/// A representative draft snapshot — a TERRAIN draft mid-edit.
fn a_draft_view() -> EditorDraftView {
    EditorDraftView::new(
        EditorModeNet::Terrain,
        vec![
            EditorDraftFieldView::new(
                EditorDraftFieldNameNet::new("display_name".to_owned()),
                EditorDraftFieldValueNet::new("Rusted Barricade".to_owned()),
            ),
            EditorDraftFieldView::new(
                EditorDraftFieldNameNet::new("kind".to_owned()),
                EditorDraftFieldValueNet::new("Cover".to_owned()),
            ),
        ],
    )
}

/// A representative validation snapshot — checks run, one dangling reference recorded.
fn a_validation_view() -> EditorValidationView {
    EditorValidationView::new(
        ValidationChecksCompleteNet::new(true),
        vec![EditorFindingView::new(
            EditorFindingKindNet::DanglingRef,
            EditorFindingSubjectNet::new("content/themes/hive.terrain_theme.ron".to_owned()),
            EditorFindingDetailNet::new(
                "unresolved 'deadbeef' in TerrainDefRegistry (UUID key)".to_owned(),
            ),
        )],
    )
}

/// Every per-topic view round-trips through compact RON identically.
#[test]
fn editor_topic_views_round_trip() {
    assert_ron_round_trip(&a_mode_view());
    assert_ron_round_trip(&a_session_view());
    assert_ron_round_trip(&a_draft_view());
    assert_ron_round_trip(&a_validation_view());
    // The `None` forms of the session's optional selections.
    assert_ron_round_trip(&EditorSessionView::new(
        EditorThemeKeyNet::new(String::new()),
        None,
        GridSizeNet::new(
            GridWidthNet::new(1),
            GridHeightNet::new(1),
            GridLevelsNet::new(1),
        ),
        None,
    ));
}

/// Every [`EditorQueryView`] answer round-trips, and the wildcard-free witness forces a new
/// topic's answer in — so the answer vocabulary cannot drift from
/// [`EditorQueryKind`](crate::view::EditorQueryKind).
#[test]
fn editor_query_view_round_trips_every_answer() {
    let answers = [
        EditorQueryView::Readiness(EditorReadinessNet::Editing),
        EditorQueryView::Mode(a_mode_view()),
        EditorQueryView::Session(a_session_view()),
        EditorQueryView::Draft(a_draft_view()),
        EditorQueryView::Validation(a_validation_view()),
    ];
    assert_eq!(answers.len(), EditorQueryKind::ALL.len());
    for answer in answers {
        match answer {
            EditorQueryView::Readiness(_)
            | EditorQueryView::Mode(_)
            | EditorQueryView::Session(_)
            | EditorQueryView::Draft(_)
            | EditorQueryView::Validation(_) => {}
        }
        assert_ron_round_trip(&EditorQueryReply::new(EditorReadinessNet::Editing, answer));
    }
}

/// The options reply round-trips with the full topic list, and every topic carries a
/// non-empty description — the discovery signal a client reads instead of the source.
#[test]
fn editor_query_options_round_trip_with_descriptions() {
    let topics: Vec<EditorQueryTopicView> = EditorQueryKind::ALL
        .into_iter()
        .map(EditorQueryTopicView::offered)
        .collect();
    for topic in &topics {
        assert!(
            !topic.description.is_empty(),
            "{:?} must carry a one-line description",
            topic.kind,
        );
    }
    assert_ron_round_trip(&EditorQueryOptionsView::new(
        EditorReadinessNet::Editing,
        topics,
    ));
    // The `Load`-phase form: readiness reported with a short topic list.
    assert_ron_round_trip(&EditorQueryOptionsView::new(
        EditorReadinessNet::Load,
        vec![EditorQueryTopicView::offered(EditorQueryKind::Readiness)],
    ));
}

/// Every readiness / mode / finding-kind variant round-trips; the witnesses force new
/// variants in rather than letting one silently read as its neighbour.
#[test]
fn editor_enums_round_trip_every_variant() {
    for readiness in [EditorReadinessNet::Load, EditorReadinessNet::Editing] {
        match readiness {
            EditorReadinessNet::Load | EditorReadinessNet::Editing => {}
        }
        assert_ron_round_trip(&readiness);
    }
    assert!(EditorReadinessNet::Editing.is_ready());
    assert!(!EditorReadinessNet::Load.is_ready());

    let modes = [
        EditorModeNet::Terrain,
        EditorModeNet::Theme,
        EditorModeNet::Prefab,
        EditorModeNet::Gang,
        EditorModeNet::Armor,
        EditorModeNet::Injury,
        EditorModeNet::Sprite,
        EditorModeNet::Attachment,
        EditorModeNet::Weapon,
        EditorModeNet::MeleeWeapon,
    ];
    for mode in modes {
        match mode {
            EditorModeNet::Terrain
            | EditorModeNet::Theme
            | EditorModeNet::Prefab
            | EditorModeNet::Gang
            | EditorModeNet::Armor
            | EditorModeNet::Injury
            | EditorModeNet::Sprite
            | EditorModeNet::Attachment
            | EditorModeNet::Weapon
            | EditorModeNet::MeleeWeapon => {}
        }
        assert_ron_round_trip(&mode);
    }

    for kind in [
        EditorFindingKindNet::DanglingRef,
        EditorFindingKindNet::MalformedFile,
        EditorFindingKindNet::DegradedFallback,
    ] {
        match kind {
            EditorFindingKindNet::DanglingRef
            | EditorFindingKindNet::MalformedFile
            | EditorFindingKindNet::DegradedFallback => {}
        }
        assert_ron_round_trip(&kind);
    }
}
