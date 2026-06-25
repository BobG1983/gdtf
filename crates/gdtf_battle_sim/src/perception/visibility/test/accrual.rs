//! AC for [`accrue`](crate::visibility::accrue): EXPLORED is monotone — after a
//! recompute where a cell leaves VISIBLE, it stays EXPLORED (accrual never removes), and
//! VISIBLE is replaced wholesale (GTW-340 clause 4 / second AC).

use super::support::*;

/// Across two successive accruals, a cell that was VISIBLE then leaves VISIBLE stays in
/// EXPLORED — and VISIBLE reflects only the latest compute.
#[test]
fn explored_is_monotone_visible_is_replaced() {
    let cell_a = key(5, 5, 0);
    let cell_b = key(9, 5, 0);

    // First compute: only cell_a is visible.
    let mut first_visible = HashSet::default();
    first_visible.insert(cell_a);
    let fog1 = accrue(&SquadVisibility::default(), first_visible);
    assert!(
        fog1.is_cell_visible(&cell_a),
        "cell_a is VISIBLE after compute 1"
    );
    assert!(
        fog1.is_cell_explored(&cell_a),
        "cell_a is EXPLORED after compute 1"
    );
    assert!(
        !fog1.is_cell_visible(&cell_b),
        "cell_b not yet VISIBLE after compute 1"
    );

    // Second compute: cell_a LEAVES visible (e.g. the observer moved away), cell_b
    // enters.
    let mut second_visible = HashSet::default();
    second_visible.insert(cell_b);
    let fog2 = accrue(&fog1, second_visible);

    // VISIBLE is replaced wholesale: cell_a is no longer VISIBLE, cell_b now is.
    assert!(
        !fog2.is_cell_visible(&cell_a),
        "cell_a left the VISIBLE set on the second compute (VISIBLE is replaced)"
    );
    assert!(
        fog2.is_cell_visible(&cell_b),
        "cell_b is VISIBLE after the second compute"
    );

    // EXPLORED is monotone: cell_a stays EXPLORED despite leaving VISIBLE, and cell_b is
    // now explored too — the set only ever grows.
    assert!(
        fog2.is_cell_explored(&cell_a),
        "cell_a remains EXPLORED after leaving VISIBLE (accrual never removes — monotone)"
    );
    assert!(
        fog2.is_cell_explored(&cell_b),
        "cell_b is EXPLORED after the second compute"
    );
}

/// An empty recompute (nothing visible this frame) preserves all prior EXPLORED memory
/// while clearing VISIBLE.
#[test]
fn empty_visible_keeps_explored_clears_visible() {
    let cell = key(7, 7, 1);
    let mut visible = HashSet::default();
    visible.insert(cell);
    let fog1 = accrue(&SquadVisibility::default(), visible);

    // Nothing visible now (e.g. all observers Downed).
    let fog2 = accrue(&fog1, HashSet::default());
    assert!(
        !fog2.is_cell_visible(&cell),
        "the cell is no longer VISIBLE after an empty recompute"
    );
    assert!(
        fog2.is_cell_explored(&cell),
        "the cell stays EXPLORED — mission memory survives an empty recompute (monotone)"
    );
}
