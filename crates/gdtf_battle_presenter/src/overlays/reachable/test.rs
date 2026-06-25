//! Pure-logic unit tests for the reachable-range read-seam + the `reachable_draws`
//! resolution (GTW-387 C3 / D3).
//!
//! The DRAW-system behaviour (the actual sprites rendered at the right cells, hard-cut
//! to the active storey) is the headless integration proof in the `gdtf_battle_input`
//! integration tests (the `tests/reachable.rs` pattern). These cover the read-seam +
//! the `reachable_draws` pure decision (active-storey hard-cut) that do not need an app.

use gdtf_battle_sim::{Cell, CellLevel, Level, Tu};

use super::overlay::{ReachableCells, reachable_draws};

/// A level-0 cell at `(x, y)`.
fn c0(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A level-1 cell at `(x, y)`.
fn c1(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(1))
}

/// The cleared reachable set is empty — no selection → nothing drawn.
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

/// `ReachableCells::new` round-trips the `(CellLevel, Tu)` pairs through [`cells`].
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

/// `reachable_draws` hard-cuts to the ACTIVE storey — cells on a different storey are
/// NOT yielded. A mixed L0 + L1 set on active level 0 yields only the L0 cells.
#[test]
fn reachable_draws_hard_cuts_to_active_storey_level_0() {
    let set = ReachableCells::new([
        (c0(1, 1), Tu::new(4)),
        (c0(2, 1), Tu::new(8)),
        (c1(3, 3), Tu::new(20)), // L1 — must be excluded when active = L0
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

/// `reachable_draws` hard-cuts to the ACTIVE storey — after a `PageUp` to L1 ONLY the L1
/// cells render. This is the key GTW-387 test: the overlay follows the level switch with
/// no extra wiring.
#[test]
fn reachable_draws_hard_cuts_to_active_storey_level_1() {
    let set = ReachableCells::new([
        (c0(1, 1), Tu::new(4)),  // L0 — must be excluded when active = L1
        (c0(2, 1), Tu::new(8)),  // L0 — same
        (c1(2, 2), Tu::new(20)), // L1 — the stair head (arrival)
        (c1(3, 2), Tu::new(24)), // L1 platform cell east
        (c1(2, 3), Tu::new(24)), // L1 platform cell south
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
    // The three specific L1 cells are present (order may vary).
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

/// `reachable_draws` on a storey with NO reachable cells is empty — the case where the
/// ganger is on L0 and the player switches to L1 with no L1 cells in the set (should
/// draw nothing rather than panicking).
#[test]
fn reachable_draws_is_empty_when_no_cells_on_active_storey() {
    let set = ReachableCells::new([(c0(1, 1), Tu::new(4)), (c0(2, 1), Tu::new(8))]);
    let draws = reachable_draws(&set, Level::new(1));
    assert!(
        draws.is_empty(),
        "no draws when the active storey has no reachable cells",
    );
}
