//! Round-trip pins for the whole-battle aggregate and its nested DTOs (GTW-734).

use crate::{
    ids::{
        CellLevelNet, CellNet, CellXNet, CellYNet, DoorToken, EmplacementToken, FireModeIndex,
        FocusTargetNet, GangerToken, LevelNet,
    },
    intent::{AimNet, FacingNet, StanceNet},
    test_support::assert_ron_round_trip,
    view::{
        BattleView, BodyPartNet, DoorOpenNet, DoorView, EmplacementMannedNet, EmplacementView,
        ExploredCellCountNet, FactionNet, FireModeLabel, FireModeView, FogView, GangerNameNet,
        GangerView, GridHeightNet, GridLevelsNet, GridSizeNet, GridWidthNet, HpMaxNet, HpNet,
        InjuryEntryNet, InjuryNameNet, InjurySummaryNet, LifeStateNet, PanelButtonLabelNet,
        PanelButtonView, PanelNavOrderNet, SelectionView, SeverityNet, TerrainSummaryView,
        TuMaxNet, TuNet, TurnView, VisibleCellCountNet, WeaponNameNet, WeaponView, WoundsMaxNet,
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

/// An empty selection round-trips (the no-selection form of the battle snapshot).
#[test]
fn empty_selection_round_trips() {
    assert_ron_round_trip(&SelectionView::new(None));
}
