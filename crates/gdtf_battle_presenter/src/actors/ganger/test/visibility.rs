//! GTW-627 classifier unit tests: the band × fog compose, the band-only absent-fog
//! branch, and the squad-fog faction-relation gate.

use bevy::{platform::collections::HashSet, prelude::*};
use gdtf_battle_sim::{
    Cell, CellLevel, Faction, FactionRelation, Level, LifeState, PlayerFaction, Position,
    SquadVisibility,
};

use super::super::visibility::{GangerFogFacts, actor_relation, classify_ganger_visibility};
use crate::{ActiveLevel, ViewMode};

/// A live player-faction ganger is `OwnSquad` (always shown); an enemy, or a corpse of
/// either faction, is `Other` (fog-gated).
#[test]
fn actor_relation_gates_by_faction_and_life() {
    let player = Faction::new(0);
    let enemy = Faction::new(1);
    let pf = Some(PlayerFaction::new(player));

    // Live player ganger -> OwnSquad.
    assert!(matches!(
        actor_relation(pf, player, LifeState::Alive),
        FactionRelation::OwnSquad
    ));
    // Live enemy ganger -> Other.
    assert!(matches!(
        actor_relation(pf, enemy, LifeState::Alive),
        FactionRelation::Other
    ));
    // A DOWNED player ganger (a corpse-like body, no longer a live observer) -> Other.
    assert!(matches!(
        actor_relation(pf, player, LifeState::Downed),
        FactionRelation::Other
    ));
    // A DEAD player ganger -> Other.
    assert!(matches!(
        actor_relation(pf, player, LifeState::Dead),
        FactionRelation::Other
    ));
    // No PlayerFaction resident (a focused harness) -> everything fog-gated (fail-closed).
    assert!(matches!(
        actor_relation(None, player, LifeState::Alive),
        FactionRelation::Other
    ));
}

/// A `SquadVisibility` whose VISIBLE (and EXPLORED — the accrual superset) set is exactly
/// `cells` — the classifier tests' fog fixture.
fn squad_seeing(cells: &[CellLevel]) -> SquadVisibility {
    let visible: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(visible.clone(), visible)
}

/// GTW-627 (P9) — the classifier's ABSENT-fog branch IS the band-only mode: with `fog`
/// `None`, faction / fog state are irrelevant and only drawn-band membership decides —
/// in-band shown (either faction), above-band hidden.
#[test]
fn classifier_without_fog_facts_is_band_only() {
    let active = ActiveLevel::new(Level::new(1));
    let in_band = Position::new(CellLevel::new(Cell::new(2, 2), Level::new(0)));
    let above = Position::new(CellLevel::new(Cell::new(3, 3), Level::new(2)));

    // In-band: shown regardless of faction — no fog fact to consult.
    assert_eq!(
        classify_ganger_visibility(
            &in_band,
            Faction::new(1),
            LifeState::Alive,
            active,
            ViewMode::DownToActive,
            None,
        ),
        Visibility::Inherited,
        "band-only mode shows ANY in-band ganger (no fog fact when the fog resources are \
         absent)",
    );
    // Strictly above the band ceiling: hidden.
    assert_eq!(
        classify_ganger_visibility(
            &above,
            Faction::new(0),
            LifeState::Alive,
            active,
            ViewMode::DownToActive,
            None,
        ),
        Visibility::Hidden,
        "band-only mode still culls a ganger strictly above the drawn band",
    );
    // FullView lifts the ceiling: the storey-2 ganger is drawn.
    assert_eq!(
        classify_ganger_visibility(
            &above,
            Faction::new(0),
            LifeState::Alive,
            active,
            ViewMode::FullView,
            None,
        ),
        Visibility::Inherited,
        "FullView draws every storey (the GTW-521 ceiling), band-only mode included",
    );
}

/// GTW-627 (C1) — with the fog facts RESIDENT the classifier ANDs both facts: an unseen
/// in-band enemy is hidden (the preserved GTW-342 hard-cut), a player ganger is shown
/// band-permitting, and an above-band player is still culled (fog cannot override band).
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

    // An in-band enemy on an UNSEEN cell: hidden by the fog fact (the hard cut, no ghost).
    assert_eq!(
        classify_ganger_visibility(
            &Position::new(unseen_at),
            enemy,
            LifeState::Alive,
            active,
            ViewMode::DownToActive,
            Some(&facts),
        ),
        Visibility::Hidden,
        "an in-band enemy on an unseen cell is hidden — the fog fact composes",
    );
    // An in-band enemy on a squad-VISIBLE cell: shown.
    assert_eq!(
        classify_ganger_visibility(
            &Position::new(seen_at),
            enemy,
            LifeState::Alive,
            active,
            ViewMode::DownToActive,
            Some(&facts),
        ),
        Visibility::Inherited,
        "an in-band enemy on a squad-VISIBLE cell is shown",
    );
    // A live player ganger on an unseen cell: shown anyway (OwnSquad is trivially visible).
    assert_eq!(
        classify_ganger_visibility(
            &Position::new(unseen_at),
            player,
            LifeState::Alive,
            active,
            ViewMode::DownToActive,
            Some(&facts),
        ),
        Visibility::Inherited,
        "a live player ganger is always shown by the fog fact (band permitting)",
    );
    // A player ganger strictly ABOVE the band: hidden — fog cannot override the band fact.
    assert_eq!(
        classify_ganger_visibility(
            &Position::new(above_at),
            player,
            LifeState::Alive,
            active,
            ViewMode::DownToActive,
            Some(&facts),
        ),
        Visibility::Hidden,
        "the band fact culls even a fog-shown player ganger above the drawn band",
    );
}
