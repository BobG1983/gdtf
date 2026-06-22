//! In-crate unit tests for the fog writer's PURE helpers: the three-state fog resolution
//! and the per-actor faction-relation gate.
//!
//! These exercise the relation / state-resolution DECISIONS in isolation (no App / no
//! render); the full system wiring (ordering after the terrain draw + cover swap, the
//! per-cell terrain DESATURATE, the actor hard-cut, the post-level-cycle re-apply) is the
//! headless integration test in `tests/fog_present.rs`. The GTW-348 per-cell
//! `TerrainFogMaterial.saturation` mapping (VISIBLE 1.0 / EXPLORED 0.0) lives in that
//! integration test, where the real material assets exist (the saturation values are not a
//! pure helper any more — they are written into the material store by the system).

use bevy::platform::collections::HashSet;
use gdtf_battle_sim::{
    Cell, CellLevel, Faction, FactionRelation, Level, LifeState, PlayerFaction, SquadVisibility,
};

use super::present::{CellFog, actor_relation};

/// `CellFog::resolve` maps the three squad states: VISIBLE wins over EXPLORED, and a cell
/// in neither set is UNSEEN.
#[test]
fn cell_fog_resolves_three_states() {
    let visible_cell = CellLevel::new(Cell::new(1, 1), Level::new(0));
    let explored_cell = CellLevel::new(Cell::new(2, 2), Level::new(0));
    let unseen_cell = CellLevel::new(Cell::new(3, 3), Level::new(0));

    // VISIBLE ⊆ EXPLORED (the accrual invariant): the visible cell is in both sets.
    let mut visible = HashSet::new();
    visible.insert(visible_cell);
    let mut explored = HashSet::new();
    explored.insert(visible_cell);
    explored.insert(explored_cell);
    let squad = SquadVisibility::new(visible, explored);

    assert!(matches!(
        CellFog::resolve(&squad, &visible_cell),
        CellFog::Visible
    ));
    assert!(matches!(
        CellFog::resolve(&squad, &explored_cell),
        CellFog::Explored
    ));
    assert!(matches!(
        CellFog::resolve(&squad, &unseen_cell),
        CellFog::Unseen
    ));
}

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
