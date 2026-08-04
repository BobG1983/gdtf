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

fn singleton(cell: CellLevel) -> HashSet<CellLevel> {
    std::iter::once(cell).collect()
}

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

#[test]
fn unseen_enemy_emits_no_threat() {
    let enemy_key = key(10, 10, 3);
    let squad = SquadVisibility::default();
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
