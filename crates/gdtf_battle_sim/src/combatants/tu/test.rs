//! the former inline `#[cfg(test)] mod tests`).

use crate::{
    ganger::{Tu, TuMax},
    tu::{TuShortfall, can_spend_tu, reset_tu, spend_tu},
};

#[test]
fn can_spend_tu_is_true_iff_pool_at_least_cost() {
    let cases = [
        (10u8, 10u8, true),
        (10, 9, true),
        (10, 11, false),
        (0, 0, true),
        (0, 1, false),
        (200, 50, true),
        (50, 200, false),
    ];
    for (pool, cost, expected) in cases {
        assert_eq!(
            *can_spend_tu(&Tu::new(pool), Tu::new(cost)),
            expected,
            "can_spend_tu(pool={pool}, cost={cost}) should be {expected}",
        );
    }
}

#[test]
fn spend_tu_over_spend_reports_the_shortfall_and_leaves_the_pool_alone() {
    let mut tu = Tu::new(5);
    let outcome = spend_tu(&mut tu, Tu::new(200));
    assert_eq!(
        outcome,
        Err(TuShortfall::new(195)),
        "an over-spend must hand the caller how far short the pool fell",
    );
    assert_eq!(
        *tu, 5,
        "an over-spend must leave the pool exactly as it was"
    );

    let mut exact = Tu::new(30);
    let spent = spend_tu(&mut exact, Tu::new(30));
    assert_eq!(spent, Ok(()), "spending the whole pool is affordable");
    assert_eq!(*exact, 0, "spending the whole pool leaves it at 0");

    let mut over_by_one = Tu::new(30);
    let short = spend_tu(&mut over_by_one, Tu::new(31));
    assert_eq!(
        short,
        Err(TuShortfall::new(1)),
        "one over the pool is short by exactly one",
    );
    assert_eq!(
        *over_by_one, 30,
        "one-over-pool must not zero the pool, and must not wrap to 255",
    );
}

#[test]
fn spend_tu_affordable_decrements_exactly() {
    let cases = [(60u8, 10u8), (60, 0), (100, 100), (7, 3)];
    for (pool, cost) in cases {
        let mut tu = Tu::new(pool);
        assert_eq!(
            spend_tu(&mut tu, Tu::new(cost)),
            Ok(()),
            "spending {cost} from {pool} is affordable",
        );
        assert_eq!(
            *tu,
            pool - cost,
            "spending {cost} from {pool} must leave exactly {} left",
            pool - cost,
        );
    }
}

#[test]
fn reset_tu_restores_pool_to_max_from_zero() {
    let max = TuMax::new(80);
    let mut tu = Tu::new(80);
    assert_eq!(
        spend_tu(&mut tu, Tu::new(80)),
        Ok(()),
        "precondition: spending the whole pool is affordable",
    );
    assert_eq!(*tu, 0, "precondition: pool drained to 0 before reset");

    reset_tu(&mut tu, &max);
    assert_eq!(*tu, *max, "reset must restore the pool to TuMax");
    assert_eq!(*tu, 80, "and the restored pool reads the max's magnitude");
}

#[test]
fn reset_tu_is_independent_of_pre_reset_pool() {
    let max = TuMax::new(50);
    for pre in [0u8, 25, 50] {
        let mut tu = Tu::new(pre);
        reset_tu(&mut tu, &max);
        assert_eq!(
            *tu, *max,
            "reset from pre={pre} must land at TuMax regardless"
        );
    }
}

#[test]
fn tu_and_tu_max_are_distinct_newtypes_read_via_deref() {
    let current = Tu::new(30);
    let max = TuMax::new(60);
    assert_eq!(*current, 30u8, "Tu derefs to its inner pool count");
    assert_eq!(*max, 60u8, "TuMax derefs to its inner round-start ceiling");
}

#[test]
fn reaction_ratio_is_in_unit_range_for_pool_within_max() {
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
