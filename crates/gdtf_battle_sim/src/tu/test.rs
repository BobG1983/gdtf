//! Relocated unit tests for the TU-economy verbs (GTW-201 wave 22 — moved verbatim from
//! the former inline `#[cfg(test)] mod tests`).

use crate::{
    ganger::{Tu, TuMax},
    tu::{can_spend_tu, reset_tu, spend_tu},
};

// AC1 — can_spend_tu(cost) is true iff the current Tu >= cost. Swept over a spread of
// (Tu, cost) pairs asserting the boundary (Tu==cost affords, Tu==cost-1 does not),
// value-agnostic on the magnitudes (the costs themselves are tuning).

#[test]
fn can_spend_tu_is_true_iff_pool_at_least_cost() {
    // (pool, cost, expected affordability) — the boundary swept around equality.
    let cases = [
        (10u8, 10u8, true), // exactly affordable: pool == cost
        (10, 9, true),      // surplus: pool > cost
        (10, 11, false),    // short by one: pool == cost - 1
        (0, 0, true),       // a zero-cost act is always affordable, even when broke
        (0, 1, false),      // a broke ganger cannot afford any positive cost
        (200, 50, true),    // large magnitudes still obey the relation
        (50, 200, false),   // and the short side, too
    ];
    for (pool, cost, expected) in cases {
        assert_eq!(
            can_spend_tu(&Tu::new(pool), Tu::new(cost)),
            expected,
            "can_spend_tu(pool={pool}, cost={cost}) should be {expected}",
        );
    }
}

// AC2 — spend_tu(cost) uses saturating arithmetic: spending more than the pool holds
// lands Tu at 0, NOT a wrap to ~255 — proving no underflow.

#[test]
fn spend_tu_over_spend_saturates_to_zero() {
    let mut tu = Tu::new(5);
    // Spend far more than is left — saturating must floor at 0, never wrap.
    spend_tu(&mut tu, Tu::new(200));
    assert_eq!(
        *tu, 0,
        "over-spending must floor the pool at 0 (no underflow wrap)"
    );

    // The exact-empty boundary: spending the whole pool leaves 0, not a wrap.
    let mut exact = Tu::new(30);
    spend_tu(&mut exact, Tu::new(30));
    assert_eq!(*exact, 0, "spending the whole pool leaves it at 0");

    // One past the boundary still floors at 0 rather than wrapping to 255.
    let mut over_by_one = Tu::new(30);
    spend_tu(&mut over_by_one, Tu::new(31));
    assert_eq!(
        *over_by_one, 0,
        "one-over-pool must floor at 0, not wrap to 255"
    );
}

// AC3 — spend_tu(cost) on an affordable cost leaves Tu == old - cost exactly.

#[test]
fn spend_tu_affordable_decrements_exactly() {
    // (pool, cost) affordable pairs — the remainder must be exactly pool - cost.
    let cases = [(60u8, 10u8), (60, 0), (100, 100), (7, 3)];
    for (pool, cost) in cases {
        let mut tu = Tu::new(pool);
        spend_tu(&mut tu, Tu::new(cost));
        assert_eq!(
            *tu,
            pool - cost,
            "spending {cost} from {pool} must leave exactly {} left",
            pool - cost,
        );
    }
}

// AC4 — reset_tu() sets Tu equal to TuMax regardless of the pre-reset Tu: drain to 0,
// reset, assert Tu == TuMax (the round-start restore).

#[test]
fn reset_tu_restores_pool_to_max_from_zero() {
    let max = TuMax::new(80);
    let mut tu = Tu::new(80);
    // Drain the pool to empty first, so the reset is provably the restorer.
    spend_tu(&mut tu, Tu::new(255));
    assert_eq!(*tu, 0, "precondition: pool drained to 0 before reset");

    reset_tu(&mut tu, &max);
    assert_eq!(*tu, *max, "reset must restore the pool to TuMax");
    assert_eq!(*tu, 80, "and the restored pool reads the max's magnitude");
}

#[test]
fn reset_tu_is_independent_of_pre_reset_pool() {
    let max = TuMax::new(50);
    // Reset from a partial pool, a full pool, and an empty pool — all land at max.
    for pre in [0u8, 25, 50] {
        let mut tu = Tu::new(pre);
        reset_tu(&mut tu, &max);
        assert_eq!(
            *tu, *max,
            "reset from pre={pre} must land at TuMax regardless"
        );
    }
}

// AC5 — TuMax is a DISTINCT newtype from Tu (no-bare-types rule 3): construct both,
// read each back via its derived Deref, and the GTW-38 reaction ratio
// (TU_left / TU_max) is computable as *Tu as f32 / *TuMax as f32 — in 0.0..=1.0 for
// Tu <= TuMax (the denominator GTW-38's reaction score reads).

#[test]
fn tu_and_tu_max_are_distinct_newtypes_read_via_deref() {
    // Distinct types over the same inner u8 — constructed separately, each read back
    // through its own derived Deref. They are never interchangeable.
    let current = Tu::new(30);
    let max = TuMax::new(60);
    assert_eq!(*current, 30u8, "Tu derefs to its inner pool count");
    assert_eq!(*max, 60u8, "TuMax derefs to its inner round-start ceiling");
}

#[test]
fn reaction_ratio_is_in_unit_range_for_pool_within_max() {
    // The GTW-38 denominator: TU_left / TU_max ∈ 0.0..=1.0 whenever Tu <= TuMax.
    // (pool, max) pairs with pool <= max, including the drained and full extremes.
    let cases = [(0u8, 60u8), (30, 60), (60, 60), (1, 200)];
    for (pool, max) in cases {
        let tu = Tu::new(pool);
        let tu_max = TuMax::new(max);
        let ratio = f32::from(*tu) / f32::from(*tu_max);
        assert!(
            (0.0..=1.0).contains(&ratio),
            "reaction ratio {ratio} (pool={pool}/max={max}) must be in 0.0..=1.0",
        );
    }
}
