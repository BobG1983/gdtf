use super::support::{
    all_other, cell, default_floor_costs, full_vision, grid_with, no_links, open_step, tuning,
};
use crate::{
    acts::{DismountSurcharge, move_tu_cost, seat_departure},
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::{Cell, CellLevel},
    pathfinder::{Departure, MoveGrids, Path, PlanningView, find_path_leaving, reachable_within},
    terrain::{
        emplacement::{EmplacementEntrySides, EmplacementFacing, emplacement_entry_cells},
        facing::TerrainFacing,
    },
};

// One step off `from` the way `side` points.
fn stepped(from: CellLevel, side: TerrainFacing) -> CellLevel {
    let step = side.cell_step();
    CellLevel::new(Cell::new(from.x + step.x, from.y + step.y), from.level())
}

#[test]
fn a_seat_departure_drops_the_cells_only_a_blocked_neighbour_reaches() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();
    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let terrain = || MoveGrids {
        occupancy:   &grid,
        links:       &links,
        floor_costs: &floor_costs,
        tuning:      &tuning,
    };

    let start = cell(5, 5, 0);
    let sides = EmplacementEntrySides::new(vec![TerrainFacing::North]);
    let facing = EmplacementFacing::new(TerrainFacing::default());
    let entry = emplacement_entry_cells(start, Some(&sides), Some(&facing)).to_vec();
    assert_eq!(
        entry.len(),
        1,
        "the fixture's seat names one entry side, so it has one entry cell: {entry:?}",
    );
    let admitted = entry.first().copied().unwrap_or(start);
    let blocked = stepped(start, TerrainFacing::South);
    let departure = seat_departure(start, start, Some(&sides), Some(&facing));
    let budget = Tu::new((*open_step(&tuning)).saturating_mul(2));

    let anywhere = reachable_within(
        &Departure::anywhere(start),
        budget,
        DismountSurcharge::NONE,
        terrain(),
        MovementCostFactor::IDENTITY,
        &planning,
    );
    let seated = reachable_within(
        &departure,
        budget,
        DismountSurcharge::NONE,
        terrain(),
        MovementCostFactor::IDENTITY,
        &planning,
    );

    assert!(
        anywhere.iter().any(|(offered, _)| *offered == blocked),
        "the fixture must discriminate: leaving anywhere, {blocked:?} is inside the budget \
         {budget:?} and must be offered; got {anywhere:?}",
    );
    assert!(
        !seated.iter().any(|(offered, _)| *offered == blocked),
        "the seat admits only {admitted:?}, so {blocked:?} costs more than {budget:?} to reach \
         and must not be offered; got {seated:?}",
    );

    for (offered, quote) in &seated {
        let charged = find_path_leaving(
            &departure,
            *offered,
            terrain(),
            MovementCostFactor::IDENTITY,
            &planning,
        )
        .ok()
        .map(|route| route.total());
        assert_eq!(
            Some(*quote),
            charged,
            "{offered:?} is offered at {quote:?}, but the route that leaves by {admitted:?} \
             charges {charged:?}",
        );
    }
}

#[test]
fn the_quote_carries_the_exit_and_the_budget_counts_it() {
    let grid = grid_with(&[]);
    let links = no_links();
    let tuning = tuning();
    let floor_costs = default_floor_costs(&tuning);
    let squad = full_vision();
    let planning = PlanningView::new(&squad, all_other);
    let terrain = || MoveGrids {
        occupancy:   &grid,
        links:       &links,
        floor_costs: &floor_costs,
        tuning:      &tuning,
    };

    let start = cell(5, 5, 0);
    let departure = Departure::anywhere(start);
    let step = *open_step(&tuning);
    let surcharge = DismountSurcharge::new(Tu::new(step.saturating_mul(2)));
    let budget = Tu::new(step.saturating_mul(3));
    // A diagonal costs more than an orthogonal step, so the exit prices the diagonal out.
    let priced_out = cell(6, 6, 0);
    let kept = cell(6, 5, 0);

    let free = reachable_within(
        &departure,
        budget,
        DismountSurcharge::NONE,
        terrain(),
        MovementCostFactor::IDENTITY,
        &planning,
    );
    let charged = reachable_within(
        &departure,
        budget,
        surcharge,
        terrain(),
        MovementCostFactor::IDENTITY,
        &planning,
    );

    assert!(
        free.iter().any(|(offered, _)| *offered == priced_out),
        "the fixture must discriminate: with no exit to pay, {priced_out:?} is inside the \
         budget {budget:?} and must be offered; got {free:?}",
    );
    assert!(
        !charged.iter().any(|(offered, _)| *offered == priced_out),
        "the exit adds {:?}, which puts {priced_out:?} over the budget {budget:?}, so it must \
         not be offered; got {charged:?}",
        *surcharge,
    );
    assert!(
        charged.iter().all(|(_, quote)| **quote <= *budget),
        "no offered cell may be quoted above the budget {budget:?}; got {charged:?}",
    );

    let quote = charged
        .iter()
        .find(|(offered, _)| *offered == kept)
        .map(|(_, quote)| *quote);
    let route = find_path_leaving(
        &departure,
        kept,
        terrain(),
        MovementCostFactor::IDENTITY,
        &planning,
    )
    .ok();
    let expected = route.as_ref().map(|path| move_tu_cost(path, surcharge));
    assert_eq!(
        quote,
        expected,
        "{kept:?} is quoted at {quote:?}; its route costs {:?} and the exit adds {:?}, which \
         sum to {expected:?}",
        route.as_ref().map(Path::total),
        *surcharge,
    );
}
