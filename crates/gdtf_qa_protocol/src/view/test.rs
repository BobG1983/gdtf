//! Round-trip pins for the read-model DTOs + exhaustive witnesses for their enums
//! (GTW-734).

use crate::{
    ids::{
        CellLevelNet, CellNet, CellXNet, CellYNet, DoorToken, EmplacementToken, FireModeIndex,
        FocusTargetNet, GangerToken, LevelNet,
    },
    intent::{AimNet, FacingNet, StanceNet, UiStackNet},
    test_support::assert_ron_round_trip,
    view::{
        AppFlowView, AppStateNet, BattleActiveNet, BattleView, BodyPartNet, CaughtUpNet,
        DoorOpenNet, DoorView, EmplacementMannedNet, EmplacementView, ExploredCellCountNet,
        FactionNet, FireModeLabel, FireModeView, FogView, GangerNameNet, GangerView, GridHeightNet,
        GridLevelsNet, GridSizeNet, GridWidthNet, HpMaxNet, HpNet, InjuryEntryNet, InjuryNameNet,
        InjurySummaryNet, LifeStateNet, MenuIdNet, MenuItemEnabledNet, MenuItemLabelNet,
        MenuItemView, MenuView, PanelButtonLabelNet, PanelButtonView, PanelNavOrderNet,
        RequestKindNet, SelectionView, SeverityNet, TerrainSummaryView, TuMaxNet, TuNet, TurnView,
        UiStackPairView, UiStackView, VisibleCellCountNet, WeaponNameNet, WeaponView, WoundsMaxNet,
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

/// A representative focus-navigable HUD button handout — an action-bar button and the
/// weapon-panel Reload, each with a focus token, Tab-chain ordinal, and label.
fn some_buttons() -> Vec<PanelButtonView> {
    vec![
        PanelButtonView::new(
            FocusTargetNet::new(400),
            PanelNavOrderNet::new(2),
            PanelButtonLabelNet::new("End Turn".to_owned()),
        ),
        PanelButtonView::new(
            FocusTargetNet::new(401),
            PanelNavOrderNet::new(100),
            PanelButtonLabelNet::new("Reload".to_owned()),
        ),
    ]
}

/// A representative whole-battle snapshot.
fn a_battle() -> BattleView {
    BattleView::new(
        vec![a_ganger()],
        a_terrain(),
        some_buttons(),
        FogView::new(VisibleCellCountNet::new(1), ExploredCellCountNet::new(1)),
        SelectionView::new(Some(GangerToken::new(100))),
        TurnView::new(FactionNet::new(0), FactionNet::new(0)),
        Some(UiStackView::new(
            UiStackPairView::new(UiStackNet::BevyUi, UiStackNet::Egui),
            UiStackNet::Egui,
        )),
    )
}

/// The whole battle snapshot — and thus every nested DTO — round-trips identically.
#[test]
fn battle_view_round_trips() {
    assert_ron_round_trip(&a_battle());
}

/// A focus-button handout (its token, ordinal, and label) round-trips identically
/// (GTW-789) — so the token a client echoes back to `SetFocus` survives the wire.
#[test]
fn panel_button_view_round_trips() {
    for button in some_buttons() {
        assert_ron_round_trip(&button);
    }
}

/// [`FogView`] is a FIXED size regardless of how much the squad can see: it carries two
/// counts, never per-cell lists. Even a fully-visible `60×60×8` grid (`28_800` cells)
/// serializes to a tiny string. This FAILS the instant anyone reverts the counts to
/// `Vec<CellLevelNet>` (a list that long serializes to tens of thousands of chars).
/// See GTW-763.
#[test]
fn fog_view_is_constant_size_regardless_of_visibility() {
    /// The `60×60×8` grid's total cell count — the ceiling a fully-visible squad reaches.
    const FULL_GRID_CELLS: u32 = 60 * 60 * 8;

    let fog = FogView::new(
        VisibleCellCountNet::new(FULL_GRID_CELLS),
        ExploredCellCountNet::new(FULL_GRID_CELLS),
    );
    let Ok(encoded) = ron::ser::to_string(&fog) else {
        unreachable!("a fog view serializes to compact RON: {fog:?}");
    };
    assert!(
        encoded.len() < 200,
        "a fully-visible fog view must stay compact (counts, not cell lists); \
         got {} chars: {encoded}",
        encoded.len()
    );
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
        None,
    ));
    assert_ron_round_trip(&AppFlowView::new(
        AppStateNet::Intro,
        BattleActiveNet::new(false),
        Vec::new(),
        CaughtUpNet::new(false),
        None,
    ));
    assert_ron_round_trip(&SelectionView::new(None));
}

/// The app-flow snapshot's folded menu view round-trips — the identity plus a mixed
/// enabled/disabled item set (GTW-787).
#[test]
fn app_flow_menu_view_round_trips() {
    let menu = MenuView::new(
        MenuIdNet::new("MainMenu".to_owned()),
        vec![
            MenuItemView::new(
                FocusTargetNet::new(11),
                MenuItemLabelNet::new("Battlescape".to_owned()),
                MenuItemEnabledNet::new(true),
            ),
            MenuItemView::new(
                FocusTargetNet::new(12),
                MenuItemLabelNet::new("HiveScape".to_owned()),
                MenuItemEnabledNet::new(false),
            ),
        ],
    );
    assert_ron_round_trip(&menu);
    assert_ron_round_trip(&AppFlowView::new(
        AppStateNet::Running,
        BattleActiveNet::new(false),
        RequestKindNet::ALL.to_vec(),
        CaughtUpNet::new(true),
        Some(menu),
    ));
}

/// Every [`RequestKindNet`] round-trips, and the witness forces new variants in — kept in
/// lock-step with the [`RequestKindNet::ALL`] table the server filters.
#[test]
fn request_kind_round_trips_every_variant() {
    assert_eq!(
        RequestKindNet::ALL.len(),
        10,
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
            | RequestKindNet::ActivateMenuItem => {}
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

/// The DEV swap-harness view round-trips both halves — the compared pair AND the live
/// stack (GTW-816). A snapshot that dropped the pair, or reported the wrong side as live,
/// fails here.
#[test]
fn ui_stack_view_round_trips() {
    let view = UiStackView::new(
        UiStackPairView::new(UiStackNet::BevyUi, UiStackNet::Egui),
        UiStackNet::Egui,
    );
    assert_ron_round_trip(&view);
    assert_eq!(view.live, UiStackNet::Egui);
    assert_eq!(view.comparison.baseline, UiStackNet::BevyUi);
}

/// A build with no swap harness reports `None` — and that absence round-trips too, so a
/// client can tell "no harness in this build" from "the `bevy_ui` stack is live".
#[test]
fn battle_view_without_a_swap_harness_round_trips() {
    let mut battle = a_battle();
    battle.ui_stack = None;
    assert_ron_round_trip(&battle);
}
