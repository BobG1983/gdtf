//! In-crate unit tests for the fog writer's PURE helper: the three-state fog resolution.
//!
//! This exercises the state-resolution DECISION in isolation (no App / no render); the
//! full system wiring (ordering after the terrain draw + cover swap, the per-cell terrain
//! DESATURATE, the post-level-cycle re-apply, the steady-frame tick-quietness) is the
//! headless integration test in `tests/fog_present/`. The GTW-348 per-cell
//! `TerrainFogMaterial.saturation` mapping (VISIBLE 1.0 / EXPLORED 0.0) lives in that
//! integration test, where the real material assets exist. The actor-side classifier
//! tests live with the GTW-627 resolver in `actors/ganger/test/visibility.rs`.

use bevy::platform::collections::HashSet;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    visibility::SquadVisibility,
};

use super::present::CellFog;

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
