//! Round-trip pins for the read-model DTOs + exhaustive witnesses for their enums
//! (GTW-734).

use crate::{
    ids::{
        CellLevelNet, CellNet, CellXNet, CellYNet, DoorToken, EmplacementToken, FireModeIndex,
        GangerToken, LevelNet,
    },
    intent::{AimNet, FacingNet, StanceNet},
    test_support::assert_ron_round_trip,
    view::{
        AppFlowView, AppStateNet, BattleActiveNet, BattleView, BodyPartNet, CaughtUpNet,
        DoorOpenNet, DoorView, EmplacementMannedNet, EmplacementView, FactionNet, FireModeLabel,
        FireModeView, FogView, GangerNameNet, GangerView, GridHeightNet, GridLevelsNet,
        GridSizeNet, GridWidthNet, HpMaxNet, HpNet, InjuryEntryNet, InjuryNameNet,
        InjurySummaryNet, LifeStateNet, RequestKindNet, SelectionView, SeverityNet,
        TerrainSummaryView, TuMaxNet, TuNet, TurnView, WeaponNameNet, WeaponView, WoundsMaxNet,
        WoundsNet,
    },
};

/// A representative cell key for building view fixtures.
fn a_key() -> CellLevelNet {
    CellLevelNet::new(
        CellNet::new(CellXNet::new(2), CellYNet::new(3)),
        LevelNet::new(1),
    )
}

/// A representative fully-populated ganger card.
fn a_ganger() -> GangerView {
    GangerView {
        token:      GangerToken::new(100),
        name:       GangerNameNet::new("Alex Mercer".to_owned()),
        faction:    FactionNet::new(0),
        position:   a_key(),
        facing:     FacingNet::East,
        aiming:     AimNet::new(true),
        stance:     StanceNet::Crouching,
        life:       LifeStateNet::Downed,
        hp:         HpNet::new(0),
        hp_max:     HpMaxNet::new(12),
        wounds:     WoundsNet::new(2),
        wounds_max: WoundsMaxNet::new(3),
        tu:         TuNet::new(5),
        tu_max:     TuMaxNet::new(10),
        injuries:   InjurySummaryNet::new(vec![InjuryEntryNet::new(
            InjuryNameNet::new("Cracked Rib".to_owned()),
            BodyPartNet::Torso,
            SeverityNet::Major,
        )]),
        weapon:     WeaponView::new(
            WeaponNameNet::new("Autogun".to_owned()),
            vec![
                FireModeView::new(
                    FireModeIndex::new(0),
                    FireModeLabel::new("Single".to_owned()),
                ),
                FireModeView::new(
                    FireModeIndex::new(1),
                    FireModeLabel::new("Burst".to_owned()),
                ),
            ],
        ),
    }
}

/// A representative terrain summary with a door + an emplacement handout.
fn a_terrain() -> TerrainSummaryView {
    TerrainSummaryView::new(
        GridSizeNet::new(
            GridWidthNet::new(60),
            GridHeightNet::new(60),
            GridLevelsNet::new(8),
        ),
        vec![DoorView::new(
            DoorToken::new(200),
            a_key(),
            DoorOpenNet::new(false),
        )],
        vec![EmplacementView::new(
            EmplacementToken::new(300),
            a_key(),
            EmplacementMannedNet::new(true),
        )],
    )
}

/// A representative whole-battle snapshot.
fn a_battle() -> BattleView {
    BattleView::new(
        vec![a_ganger()],
        a_terrain(),
        FogView::new(vec![a_key()], vec![a_key()]),
        SelectionView::new(Some(GangerToken::new(100))),
        TurnView::new(FactionNet::new(0), FactionNet::new(0)),
    )
}

/// The whole battle snapshot — and thus every nested DTO — round-trips identically.
#[test]
fn battle_view_round_trips() {
    assert_ron_round_trip(&a_battle());
}

/// The app-flow snapshot (with a populated affordance list) and an empty selection
/// round-trip; the empty-affordance form round-trips too.
#[test]
fn app_flow_and_empty_selection_round_trip() {
    assert_ron_round_trip(&AppFlowView::new(
        AppStateNet::Running,
        BattleActiveNet::new(true),
        RequestKindNet::ALL.to_vec(),
        CaughtUpNet::new(true),
    ));
    assert_ron_round_trip(&AppFlowView::new(
        AppStateNet::Intro,
        BattleActiveNet::new(false),
        Vec::new(),
        CaughtUpNet::new(false),
    ));
    assert_ron_round_trip(&SelectionView::new(None));
}

/// Every [`RequestKindNet`] round-trips, and the witness forces new variants in — kept in
/// lock-step with the [`RequestKindNet::ALL`] table the server filters.
#[test]
fn request_kind_round_trips_every_variant() {
    assert_eq!(
        RequestKindNet::ALL.len(),
        8,
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
            | RequestKindNet::StartBattle => {}
        }
        assert_ron_round_trip(&kind);
    }
}

/// Every [`LifeStateNet`] variant round-trips; the witness forces new variants in.
#[test]
fn life_state_round_trips_every_variant() {
    for life in [
        LifeStateNet::Alive,
        LifeStateNet::Downed,
        LifeStateNet::Dead,
    ] {
        match life {
            LifeStateNet::Alive | LifeStateNet::Downed | LifeStateNet::Dead => {}
        }
        assert_ron_round_trip(&life);
    }
}

/// Every [`SeverityNet`] bucket round-trips; the witness forces new variants in.
#[test]
fn severity_round_trips_every_variant() {
    for severity in [
        SeverityNet::None,
        SeverityNet::Minor,
        SeverityNet::Major,
        SeverityNet::Critical,
        SeverityNet::Fatal,
    ] {
        match severity {
            SeverityNet::None
            | SeverityNet::Minor
            | SeverityNet::Major
            | SeverityNet::Critical
            | SeverityNet::Fatal => {}
        }
        assert_ron_round_trip(&severity);
    }
}

/// Every [`BodyPartNet`] round-trips; the witness forces new variants in.
#[test]
fn body_part_round_trips_every_variant() {
    for part in [
        BodyPartNet::Head,
        BodyPartNet::Torso,
        BodyPartNet::LeftArm,
        BodyPartNet::RightArm,
        BodyPartNet::LeftLeg,
        BodyPartNet::RightLeg,
    ] {
        match part {
            BodyPartNet::Head
            | BodyPartNet::Torso
            | BodyPartNet::LeftArm
            | BodyPartNet::RightArm
            | BodyPartNet::LeftLeg
            | BodyPartNet::RightLeg => {}
        }
        assert_ron_round_trip(&part);
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
