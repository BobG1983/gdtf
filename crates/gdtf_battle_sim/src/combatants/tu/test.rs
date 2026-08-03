//! the former inline `#[cfg(test)] mod tests`).

use crate::{
    ganger::{Tu, TuMax},
    tu::{can_spend_tu, reset_tu, spend_tu},
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
fn spend_tu_over_spend_saturates_to_zero() {
    let mut tu = Tu::new(5);
    spend_tu(&mut tu, Tu::new(200));
    assert_eq!(
        *tu, 0,
        "over-spending must floor the pool at 0 (no underflow wrap)"
    );

    let mut exact = Tu::new(30);
    spend_tu(&mut exact, Tu::new(30));
    assert_eq!(*exact, 0, "spending the whole pool leaves it at 0");

    let mut over_by_one = Tu::new(30);
    spend_tu(&mut over_by_one, Tu::new(31));
    assert_eq!(
        *over_by_one, 0,
        "one-over-pool must floor at 0, not wrap to 255"
    );
}


#[test]
fn spend_tu_affordable_decrements_exactly() {
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


#[test]
fn reset_tu_restores_pool_to_max_from_zero() {
    let max = TuMax::new(80);
    let mut tu = Tu::new(80);
    spend_tu(&mut tu, Tu::new(255));
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
