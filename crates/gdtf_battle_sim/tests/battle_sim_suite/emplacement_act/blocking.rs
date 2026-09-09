//! A seeded emplacement blocks the route across its cell and the sightline through it.

use bevy::app::App;
use gdtf_battle_sim::{
    floor::FloorCostGrid,
    ganger::Direction,
    injuries::MovementCostFactor,
    los::Sighted,
    metric::CellLevel,
    occupancy::OccupancyGrid,
    pathfinder::{MoveGrids, PlanningView, find_path},
    test_support::{SituationBuilder, emplacement_at},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

use super::harness::*;

/// The cell the emplacement is seeded on, between the two gangers.
fn mount_cell() -> CellLevel {
    ground(6, 5)
}

/// Where the route starts, one step west of the mount and on one of its entry sides.
fn west_cell() -> CellLevel {
    ground(5, 5)
}

/// Where the second ganger stands, so the cells east of the mount are explored.
fn east_cell() -> CellLevel {
    ground(9, 5)
}

/// Where the route ends, one step east of the mount.
fn goal_cell() -> CellLevel {
    ground(7, 5)
}

/// The two cells a route from [`west_cell`] to [`goal_cell`] can detour through.
fn detour_cells() -> [CellLevel; 2] {
    [ground(6, 4), ground(6, 6)]
}

/// Two standing player gangers either side of one seeded emplacement.
fn blocked_layout(seed: u64) -> App {
    let (mut app, seed) = battle_app(seed);
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(west_cell(), Direction::East),
            player_at(east_cell(), Direction::West),
        ])
        .with_scatter(emplacement_at(mount_cell()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    app
}

/// The cells A* routes through, or `None` when no route reached the goal.
fn route_cells(app: &App, start: CellLevel, goal: CellLevel) -> Option<Vec<CellLevel>> {
    let world = app.world();
    let (Some(occupancy), Some(links), Some(floor_costs), Some(tuning), Some(squad)) = (
        world.get_resource::<OccupancyGrid>(),
        world.get_resource::<VerticalLinkGraph>(),
        world.get_resource::<FloorCostGrid>(),
        world.get_resource::<CombatTuning>(),
        world.get_resource::<SquadVisibility>(),
    ) else {
        unreachable!("setup inserts the grids, the tuning and the fog the planner reads");
    };
    let planning = PlanningView::new(squad, |_occupant| FactionRelation::Other);
    find_path(
        start,
        goal,
        MoveGrids {
            occupancy,
            links,
            floor_costs,
            tuning,
        },
        MovementCostFactor::IDENTITY,
        &planning,
    )
    .ok()
    .map(|path| path.cells().to_vec())
}

/// Whether the squad has explored a cell, so the planner will route through it.
fn explored(app: &App, at: CellLevel) -> bool {
    let Some(squad) = app.world().get_resource::<SquadVisibility>() else {
        unreachable!("setup inserts the SquadVisibility the planner reads its fog from");
    };
    *squad.is_cell_explored(&at)
}

#[test]
fn a_route_across_a_seeded_emplacement_steps_around_its_cell() {
    let app = blocked_layout(0x5543_1185);

    assert!(
        explored(&app, goal_cell()),
        "PRECONDITION: the goal {:?} is unexplored, so the planner refuses it as unroutable and \
         no route can reach it whatever the mount blocks",
        goal_cell(),
    );
    assert!(
        detour_cells().iter().any(|at| explored(&app, *at)),
        "PRECONDITION: neither detour cell {:?} is explored, so the route around the mount was \
         never available to the planner",
        detour_cells(),
    );

    let Some(cells) = route_cells(&app, west_cell(), goal_cell()) else {
        unreachable!(
            "no route reached the goal: the planner found nothing from {:?} to {:?}",
            west_cell(),
            goal_cell(),
        );
    };
    assert!(
        !cells.contains(&mount_cell()),
        "the route stepped onto the mount at {:?} instead of around it: {cells:?}",
        mount_cell(),
    );

    let Some(grid) = app.world().get_resource::<OccupancyGrid>() else {
        unreachable!("setup inserts the OccupancyGrid the path surface is written into");
    };
    assert!(
        *grid.is_path_blocked(&mount_cell()),
        "the mount's cell {:?} is off the path surface, so nothing made the route detour",
        mount_cell(),
    );
}

#[test]
fn standing_gangers_either_side_of_a_seeded_emplacement_do_not_see_each_other() {
    let mut app = blocked_layout(0x5543_1285);
    high_mount_band(&app, mount_cell());

    let west = player_on(&mut app, west_cell());
    let east = player_on(&mut app, east_cell());

    assert_eq!(
        sighted(&app, west, east),
        Sighted::new(false),
        "the ganger on {:?} sees through the mount on {:?} to the ganger on {:?}",
        west_cell(),
        mount_cell(),
        east_cell(),
    );
    assert_eq!(
        sighted(&app, east, west),
        Sighted::new(false),
        "the ganger on {:?} sees through the mount on {:?} to the ganger on {:?}",
        east_cell(),
        mount_cell(),
        west_cell(),
    );
}

#[test]
fn a_seeded_emplacement_puts_its_own_band_on_the_vision_surface() {
    let app = blocked_layout(0x5543_1385);
    let band = high_mount_band(&app, mount_cell());

    let Some(grid) = app.world().get_resource::<OccupancyGrid>() else {
        unreachable!("setup inserts the OccupancyGrid the vision surface is written into");
    };
    let occluder = grid.vision_occluder_at(&mount_cell());
    assert_eq!(
        occluder,
        Some(band),
        "the mount's cell {:?} carries no vision occluder at the band its def derives, {band:?}",
        mount_cell(),
    );
}
