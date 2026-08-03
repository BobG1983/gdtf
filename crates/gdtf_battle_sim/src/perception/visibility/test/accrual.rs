use super::support::*;

#[test]
fn explored_is_monotone_visible_is_replaced() {
    let cell_a = key(5, 5, 0);
    let cell_b = key(9, 5, 0);

    let mut first_visible = HashSet::default();
    first_visible.insert(cell_a);
    let fog1 = accrue(&SquadVisibility::default(), first_visible);
    assert!(
        *fog1.is_cell_visible(&cell_a),
        "cell_a is VISIBLE after compute 1"
    );
    assert!(
        *fog1.is_cell_explored(&cell_a),
        "cell_a is EXPLORED after compute 1"
    );
    assert!(
        !*fog1.is_cell_visible(&cell_b),
        "cell_b not yet VISIBLE after compute 1"
    );

    let mut second_visible = HashSet::default();
    second_visible.insert(cell_b);
    let fog2 = accrue(&fog1, second_visible);

    assert!(
        !*fog2.is_cell_visible(&cell_a),
        "cell_a left the VISIBLE set on the second compute (VISIBLE is replaced)"
    );
    assert!(
        *fog2.is_cell_visible(&cell_b),
        "cell_b is VISIBLE after the second compute"
    );

    assert!(
        *fog2.is_cell_explored(&cell_a),
        "cell_a remains EXPLORED after leaving VISIBLE (accrual never removes — monotone)"
    );
    assert!(
        *fog2.is_cell_explored(&cell_b),
        "cell_b is EXPLORED after the second compute"
    );
}

#[test]
fn empty_visible_keeps_explored_clears_visible() {
    let cell = key(7, 7, 1);
    let mut visible = HashSet::default();
    visible.insert(cell);
    let fog1 = accrue(&SquadVisibility::default(), visible);

    let fog2 = accrue(&fog1, HashSet::default());
    assert!(
        !*fog2.is_cell_visible(&cell),
        "the cell is no longer VISIBLE after an empty recompute"
    );
    assert!(
        *fog2.is_cell_explored(&cell),
        "the cell stays EXPLORED — mission memory survives an empty recompute (monotone)"
    );
}
