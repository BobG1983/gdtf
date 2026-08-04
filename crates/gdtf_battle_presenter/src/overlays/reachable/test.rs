use gdtf_battle_sim::prelude::{Cell, CellLevel, Level, Tu};

use super::overlay::{ReachableCells, reachable_draws};

fn c0(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn c1(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(1))
}

#[test]
fn cleared_reachable_set_is_empty() {
    let set = ReachableCells::cleared();
    assert!(set.is_empty(), "the cleared reachable set holds no cells");
    let draws = reachable_draws(&set, Level::new(0));
    assert!(
        draws.is_empty(),
        "the cleared set produces no draws on any storey",
    );
}

#[test]
fn new_reachable_set_holds_pairs() {
    let pairs = vec![(c0(1, 2), Tu::new(4)), (c1(3, 4), Tu::new(8))];
    let set = ReachableCells::new(pairs.iter().copied());
    let got: Vec<(CellLevel, Tu)> = set.cells().collect();
    assert_eq!(
        got, pairs,
        "the reachable set round-trips the (cell, cost) pairs",
    );
}

#[test]
fn reachable_draws_hard_cuts_to_active_storey_level_0() {
    let set = ReachableCells::new([
        (c0(1, 1), Tu::new(4)),
        (c0(2, 1), Tu::new(8)),
        (c1(3, 3), Tu::new(20)),
    ]);
    let draws = reachable_draws(&set, Level::new(0));
    assert_eq!(
        draws,
        vec![c0(1, 1), c0(2, 1)],
        "only L0 cells are drawn when the active level is 0",
    );
    assert!(
        !draws.iter().any(|c| c.z != 0),
        "no L1 cell leaks into the active-L0 draw set",
    );
}

#[test]
fn reachable_draws_hard_cuts_to_active_storey_level_1() {
    let set = ReachableCells::new([
        (c0(1, 1), Tu::new(4)),
        (c0(2, 1), Tu::new(8)),
        (c1(2, 2), Tu::new(20)),
        (c1(3, 2), Tu::new(24)),
        (c1(2, 3), Tu::new(24)),
    ]);
    let draws = reachable_draws(&set, Level::new(1));
    assert_eq!(
        draws.len(),
        3,
        "three L1 cells draw on the active L1 storey (the stair head + 2 platform cells)",
    );
    assert!(
        draws.iter().all(|c| c.z == 1),
        "ALL draws are on L1 when active level is 1 — no L0 leaks through",
    );
    assert!(
        draws.contains(&c1(2, 2)),
        "the stair head (2,2,L1) is in the L1 draws",
    );
    assert!(
        draws.contains(&c1(3, 2)),
        "the east platform cell (3,2,L1) is in the L1 draws",
    );
    assert!(
        draws.contains(&c1(2, 3)),
        "the south platform cell (2,3,L1) is in the L1 draws",
    );
}

#[test]
fn reachable_draws_is_empty_when_no_cells_on_active_storey() {
    let set = ReachableCells::new([(c0(1, 1), Tu::new(4)), (c0(2, 1), Tu::new(8))]);
    let draws = reachable_draws(&set, Level::new(1));
    assert!(
        draws.is_empty(),
        "no draws when the active storey has no reachable cells",
    );
}
