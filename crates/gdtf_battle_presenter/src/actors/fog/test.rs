use bevy::platform::collections::HashSet;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    visibility::SquadVisibility,
};

use super::present::CellFog;

#[test]
fn cell_fog_resolves_three_states() {
    let visible_cell = CellLevel::new(Cell::new(1, 1), Level::new(0));
    let explored_cell = CellLevel::new(Cell::new(2, 2), Level::new(0));
    let unseen_cell = CellLevel::new(Cell::new(3, 3), Level::new(0));

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
