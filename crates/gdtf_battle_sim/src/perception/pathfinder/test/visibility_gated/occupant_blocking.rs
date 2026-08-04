use super::{
    super::support::{cell, default_floor_costs, fog, grid_with, no_links, tuning},
    support::*,
};
use crate::{
    occupancy::TerrainKind,
    pathfinder::{MoveGrids, PlanningView, find_path},
    visibility::FactionRelation,
};

#[test]
fn visible_enemy_blocks_the_route() {
    let mut grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let enemy = spawn_entity();
    let block_cell = cell(2, 5, 0);
    grid.set_occupant(block_cell, Some(enemy));
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&corridor_cells(), &[]);
    let planning = PlanningView::new(&squad, resolve_as(enemy, FactionRelation::Other));
    let result = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert!(
        result.is_err(),
        "a squad-VISIBLE enemy on the only route blocks it (PathBlocked), got {result:?}",
    );

    let clear_grid = corridor();
    let open = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &clear_grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert!(
        open.is_ok(),
        "with no occupant the corridor is open — the block was the visible enemy, got {open:?}",
    );
}

#[test]
fn invisible_enemy_does_not_block() {
    let mut grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let enemy = spawn_entity();
    let block_cell = cell(2, 5, 0);
    grid.set_occupant(block_cell, Some(enemy));
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&[], &corridor_cells());
    let planning = PlanningView::new(&squad, resolve_as(enemy, FactionRelation::Other));
    let result = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert!(
        result.is_ok(),
        "an enemy the squad cannot see must NOT block the route, got {result:?}",
    );
    assert!(
        result.is_ok_and(|p| p.cells().contains(&block_cell)),
        "the route passes THROUGH the invisible enemy's cell (no fog-leaking detour)",
    );
}

#[test]
fn own_squad_ganger_always_blocks() {
    let mut grid = corridor();
    let links = no_links();
    let tuning = tuning();
    let mate = spawn_entity();
    let block_cell = cell(2, 5, 0);
    grid.set_occupant(block_cell, Some(mate));
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&[], &corridor_cells());
    let planning = PlanningView::new(&squad, resolve_as(mate, FactionRelation::OwnSquad));
    let result = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert!(
        result.is_err(),
        "an own-squad ganger always blocks, even on an EXPLORED-only cell, got {result:?}",
    );
}

#[test]
fn explored_scatter_blocks_the_route() {
    let links = no_links();
    let tuning = tuning();
    let occupant = spawn_entity();
    let block_cell = cell(2, 5, 0);
    let start = cell(0, 5, 0);
    let goal = cell(4, 5, 0);

    let mut walls = Vec::new();
    for x in 0..=4 {
        walls.push((cell(x, 4, 0), TerrainKind::Wall));
        walls.push((cell(x, 6, 0), TerrainKind::Wall));
    }
    walls.push((block_cell, TerrainKind::Cover));
    let grid = grid_with(&walls);

    let floor_costs = default_floor_costs(&tuning);
    let squad = fog(&[], &corridor_cells());
    let planning = PlanningView::new(&squad, resolve_as(occupant, FactionRelation::Other));
    let result = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert!(
        result.is_err(),
        "a blocking scatter (Cover) on the only route blocks it even when explored, got \
         {result:?}",
    );

    let open_grid = corridor();
    let open = find_path(
        start,
        goal,
        MoveGrids {
            occupancy:   &open_grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        crate::injuries::MovementCostFactor::IDENTITY,
        &planning,
    );
    assert!(
        open.is_ok(),
        "with the scatter removed the explored corridor is open — the block was the Cover, got \
         {open:?}",
    );
}
