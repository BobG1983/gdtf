use bevy::{platform::collections::HashSet, prelude::*};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position},
    visibility::{FactionRelation, SquadVisibility},
};

use super::super::visibility::{GangerFogFacts, actor_relation, classify_ganger_visibility};
use crate::{ActiveLevel, IsolateView, StoreyViewMode, ViewMode};

fn mode(view: ViewMode) -> StoreyViewMode {
    StoreyViewMode::new(view, IsolateView::Off)
}

#[test]
fn actor_relation_gates_by_faction_and_life() {
    let player = Faction::new(0);
    let enemy = Faction::new(1);
    let pf = Some(PlayerFaction::new(player));

    assert!(matches!(
        actor_relation(pf, player, LifeState::Alive),
        FactionRelation::OwnSquad
    ));
    assert!(matches!(
        actor_relation(pf, enemy, LifeState::Alive),
        FactionRelation::Other
    ));
    assert!(matches!(
        actor_relation(pf, player, LifeState::Downed),
        FactionRelation::Other
    ));
    assert!(matches!(
        actor_relation(pf, player, LifeState::Dead),
        FactionRelation::Other
    ));
    assert!(matches!(
        actor_relation(None, player, LifeState::Alive),
        FactionRelation::Other
    ));
}

fn squad_seeing(cells: &[CellLevel]) -> SquadVisibility {
    let visible: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(visible.clone(), visible)
}

#[test]
fn classifier_without_fog_facts_is_band_only() {
    let active = ActiveLevel::new(Level::new(1));
    let in_band = Position::new(CellLevel::new(Cell::new(2, 2), Level::new(0)));
    let above = Position::new(CellLevel::new(Cell::new(3, 3), Level::new(2)));

    assert_eq!(
        classify_ganger_visibility(
            &in_band,
            Faction::new(1),
            LifeState::Alive,
            active,
            mode(ViewMode::DownToActive),
            None,
        ),
        Visibility::Inherited,
        "band-only mode shows ANY in-band ganger (no fog fact when the fog resources are \
         absent)",
    );
    assert_eq!(
        classify_ganger_visibility(
            &above,
            Faction::new(0),
            LifeState::Alive,
            active,
            mode(ViewMode::DownToActive),
            None,
        ),
        Visibility::Hidden,
        "band-only mode still culls a ganger strictly above the drawn band",
    );
    assert_eq!(
        classify_ganger_visibility(
            &above,
            Faction::new(0),
            LifeState::Alive,
            active,
            mode(ViewMode::FullView),
            None,
        ),
        Visibility::Inherited,
        "FullView draws every storey (the GTW-521 ceiling), band-only mode included",
    );
}

#[test]
fn classifier_with_fog_facts_composes_band_and_fog() {
    let active = ActiveLevel::new(Level::new(1));
    let player = Faction::new(0);
    let enemy = Faction::new(1);
    let seen_at = CellLevel::new(Cell::new(2, 2), Level::new(0));
    let unseen_at = CellLevel::new(Cell::new(9, 9), Level::new(0));
    let above_at = CellLevel::new(Cell::new(2, 2), Level::new(2));
    let squad = squad_seeing(&[seen_at]);
    let facts = GangerFogFacts::new(&squad, Some(PlayerFaction::new(player)));

    assert_eq!(
        classify_ganger_visibility(
            &Position::new(unseen_at),
            enemy,
            LifeState::Alive,
            active,
            mode(ViewMode::DownToActive),
            Some(&facts),
        ),
        Visibility::Hidden,
        "an in-band enemy on an unseen cell is hidden — the fog fact composes",
    );
    assert_eq!(
        classify_ganger_visibility(
            &Position::new(seen_at),
            enemy,
            LifeState::Alive,
            active,
            mode(ViewMode::DownToActive),
            Some(&facts),
        ),
        Visibility::Inherited,
        "an in-band enemy on a squad-VISIBLE cell is shown",
    );
    assert_eq!(
        classify_ganger_visibility(
            &Position::new(unseen_at),
            player,
            LifeState::Alive,
            active,
            mode(ViewMode::DownToActive),
            Some(&facts),
        ),
        Visibility::Inherited,
        "a live player ganger is always shown by the fog fact (band permitting)",
    );
    assert_eq!(
        classify_ganger_visibility(
            &Position::new(above_at),
            player,
            LifeState::Alive,
            active,
            mode(ViewMode::DownToActive),
            Some(&facts),
        ),
        Visibility::Hidden,
        "the band fact culls even a fog-shown player ganger above the drawn band",
    );
}
