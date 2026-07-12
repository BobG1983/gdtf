//! GTW-596 Threat gathering: the fog-gating invariant (BOTH directions, on the
//! real `is_ganger_visible` path — acceptance clause 1) + the per-producer
//! positive assertions for `ThreatAbove` / `ThreatBelow` (acceptance clause 2).

use bevy::platform::collections::HashSet;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position},
    visibility::SquadVisibility,
};

use crate::overlays::cross_level_signals::{threat::gather_threats, types::LevelDelta};

fn key(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

fn player() -> PlayerFaction {
    PlayerFaction::new(Faction::new(0))
}

/// A one-cell squad set — the common single-`CellLevel` fixture every test below builds.
fn singleton(cell: CellLevel) -> HashSet<CellLevel> {
    std::iter::once(cell).collect()
}

/// Acceptance clause 2 (`ThreatAbove`): a LIVE enemy whose actual `(cell, level)`
/// is squad-VISIBLE, on a storey ABOVE the active one, emits a Threat entry with
/// a POSITIVE delta — clause 1's positive direction too.
#[test]
fn visible_enemy_above_emits_threat_above() {
    let enemy_key = key(10, 10, 3);
    let visible: HashSet<CellLevel> = singleton(enemy_key);
    let squad = SquadVisibility::new(visible.clone(), visible);
    let pos = Position::new(enemy_key);
    let faction = Faction::new(1);
    let life = LifeState::Alive;
    let gangers = [(&pos, &faction, &life)];

    let threats = gather_threats(gangers.into_iter(), &squad, Some(player()), Level::new(0));

    assert_eq!(
        threats,
        vec![(Cell::new(10, 10), LevelDelta::new(3))],
        "a squad-VISIBLE enemy 3 storeys above the active level emits ThreatAbove(+3)",
    );
}

/// Acceptance clause 2 (`ThreatBelow`): the mirror case — an enemy BELOW the
/// active storey emits a NEGATIVE delta.
#[test]
fn visible_enemy_below_emits_threat_below() {
    let enemy_key = key(4, 4, 0);
    let visible: HashSet<CellLevel> = singleton(enemy_key);
    let squad = SquadVisibility::new(visible.clone(), visible);
    let pos = Position::new(enemy_key);
    let faction = Faction::new(1);
    let life = LifeState::Alive;
    let gangers = [(&pos, &faction, &life)];

    let threats = gather_threats(gangers.into_iter(), &squad, Some(player()), Level::new(2));

    assert_eq!(
        threats,
        vec![(Cell::new(4, 4), LevelDelta::new(-2))],
        "a squad-VISIBLE enemy 2 storeys below the active level emits ThreatBelow(-2)",
    );
}

/// Acceptance clause 1 (negative, UNSEEN): an enemy whose cell is in NEITHER the
/// VISIBLE nor the EXPLORED set emits NOTHING — the load-bearing fog-gating
/// invariant (never leak an UNSEEN enemy).
#[test]
fn unseen_enemy_emits_no_threat() {
    let enemy_key = key(10, 10, 3);
    let squad = SquadVisibility::default(); // neither set holds the enemy's cell
    let pos = Position::new(enemy_key);
    let faction = Faction::new(1);
    let life = LifeState::Alive;
    let gangers = [(&pos, &faction, &life)];

    let threats = gather_threats(gangers.into_iter(), &squad, Some(player()), Level::new(0));

    assert!(
        threats.is_empty(),
        "an UNSEEN cross-level enemy must never leak a threat badge",
    );
}

/// Acceptance clause 1 (negative, EXPLORED-only): an enemy whose cell is
/// EXPLORED but NOT currently VISIBLE ALSO emits nothing — `is_ganger_visible`
/// gates on VISIBLE, not EXPLORED (the contract's exact wording).
#[test]
fn explored_only_enemy_emits_no_threat() {
    let enemy_key = key(10, 10, 3);
    let explored: HashSet<CellLevel> = singleton(enemy_key);
    let squad = SquadVisibility::new(HashSet::default(), explored);
    let pos = Position::new(enemy_key);
    let faction = Faction::new(1);
    let life = LifeState::Alive;
    let gangers = [(&pos, &faction, &life)];

    let threats = gather_threats(gangers.into_iter(), &squad, Some(player()), Level::new(0));

    assert!(
        threats.is_empty(),
        "an EXPLORED-only (not currently visible) cross-level enemy must never \
         leak a threat badge",
    );
}

/// A Downed enemy poses no "threat" even when squad-VISIBLE above.
#[test]
fn incapacitated_enemy_emits_no_threat() {
    let enemy_key = key(10, 10, 3);
    let visible: HashSet<CellLevel> = singleton(enemy_key);
    let squad = SquadVisibility::new(visible.clone(), visible);
    let pos = Position::new(enemy_key);
    let faction = Faction::new(1);
    let life = LifeState::Downed;
    let gangers = [(&pos, &faction, &life)];

    let threats = gather_threats(gangers.into_iter(), &squad, Some(player()), Level::new(0));

    assert!(threats.is_empty(), "a Downed enemy is not a 'threat'");
}

/// An own-squad ganger on another storey never surfaces as a "threat" (Threat is
/// enemy-only).
#[test]
fn own_squad_member_emits_no_threat() {
    let ally_key = key(10, 10, 3);
    let visible: HashSet<CellLevel> = singleton(ally_key);
    let squad = SquadVisibility::new(visible.clone(), visible);
    let pos = Position::new(ally_key);
    let faction = Faction::new(0);
    let life = LifeState::Alive;
    let gangers = [(&pos, &faction, &life)];

    let threats = gather_threats(gangers.into_iter(), &squad, Some(player()), Level::new(0));

    assert!(threats.is_empty(), "an own-squad member is never a threat");
}

/// A same-storey enemy (no cross-level relationship) emits nothing — it is drawn
/// as an ordinary sprite, not a badge.
#[test]
fn same_storey_enemy_emits_no_threat() {
    let enemy_key = key(10, 10, 0);
    let visible: HashSet<CellLevel> = singleton(enemy_key);
    let squad = SquadVisibility::new(visible.clone(), visible);
    let pos = Position::new(enemy_key);
    let faction = Faction::new(1);
    let life = LifeState::Alive;
    let gangers = [(&pos, &faction, &life)];

    let threats = gather_threats(gangers.into_iter(), &squad, Some(player()), Level::new(0));

    assert!(threats.is_empty(), "a same-storey enemy is not cross-level");
}
