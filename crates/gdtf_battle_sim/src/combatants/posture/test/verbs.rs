//! verbatim from the former inline `#[cfg(test)] mod tests`).

use crate::{
    ganger::{Aiming, Direction, Facing, RingSteps, Stance, StanceKind, Tu},
    posture::{set_aiming, set_facing, set_stance},
    tuning::{StanceChangeTu, TurnTu},
};

#[test]
fn set_aiming_toggles_flag_and_charges_no_tu() {
    let mut aiming = Aiming::new(false);
    let tu = Tu::new(30);

    set_aiming(&mut aiming, Aiming::new(true));
    assert!(*aiming, "set_aiming(true) must set the aim flag");
    assert_eq!(*tu, 30, "toggling aim must not spend any TU");

    set_aiming(&mut aiming, Aiming::new(false));
    assert!(!*aiming, "set_aiming(false) must clear the aim flag");
    assert_eq!(*tu, 30, "toggling aim off must not spend any TU either");
}

#[test]
fn set_stance_to_different_stance_spends_exactly_the_cost() {
    let cost = StanceChangeTu::new(8);
    let mut stance = Stance::new(StanceKind::Standing);
    let mut tu = Tu::new(60);
    let before = *tu;

    let changed = *set_stance(&mut stance, &mut tu, StanceKind::Prone, &cost);

    assert!(
        changed,
        "changing to a different stance must report a change"
    );
    assert_eq!(
        *stance,
        StanceKind::Prone,
        "the new stance must be set after a real change",
    );
    assert!(
        *tu < before,
        "a real stance change must strictly decrease TU"
    );
    assert_eq!(
        before - *tu,
        *cost,
        "the TU drop must equal exactly the StanceChangeTu tuning leaf",
    );
}

#[test]
fn set_stance_to_same_stance_is_a_no_op() {
    let cost = StanceChangeTu::new(8);
    let mut stance = Stance::new(StanceKind::Crouching);
    let mut tu = Tu::new(60);

    let changed = *set_stance(&mut stance, &mut tu, StanceKind::Crouching, &cost);

    assert!(
        !changed,
        "re-asserting the held stance must report no change"
    );
    assert_eq!(
        *stance,
        StanceKind::Crouching,
        "the stance is unchanged on a no-op",
    );
    assert_eq!(
        *tu, 60,
        "re-asserting the held stance must not spend any TU"
    );
}

#[test]
fn set_facing_to_different_facing_spends_exactly_the_cost() {
    let cost = TurnTu::new(4);

    let mut facing = Facing::new(Direction::North);
    let mut tu = Tu::new(50);
    let before = *tu;

    let changed = *set_facing(&mut facing, &mut tu, Direction::East, &cost);

    assert!(
        changed,
        "turning to a different facing must report a change"
    );
    assert_eq!(
        *facing,
        Direction::East,
        "a fully-affordable turn lands on the requested facing",
    );
    assert!(*tu < before, "a real turn must strictly decrease TU");
    assert_eq!(
        before - *tu,
        *Direction::North.steps_to(Direction::East) * (*cost),
        "the TU drop must equal exactly (short-way steps) * the per-step TurnTu leaf",
    );

    let mut facing = Facing::new(Direction::North);
    let mut tu = Tu::new(50);
    let before = *tu;

    let changed = *set_facing(&mut facing, &mut tu, Direction::South, &cost);

    assert!(
        changed,
        "an opposite turn on an ample pool reports a change"
    );
    assert_eq!(
        *facing,
        Direction::South,
        "a fully-affordable opposite turn lands on the requested facing",
    );
    assert_eq!(
        before - *tu,
        *Direction::North.steps_to(Direction::South) * (*cost),
        "an opposite turn costs steps_to (= 4) * the per-step TurnTu leaf",
    );
}

#[test]
fn set_facing_to_same_facing_is_a_no_op() {
    let cost = TurnTu::new(4);
    let mut facing = Facing::new(Direction::SouthWest);
    let mut tu = Tu::new(50);

    let changed = *set_facing(&mut facing, &mut tu, Direction::SouthWest, &cost);

    assert!(
        !changed,
        "re-asserting the held facing must report no change"
    );
    assert_eq!(
        *facing,
        Direction::SouthWest,
        "the facing is unchanged on a no-op",
    );
    assert_eq!(
        *tu, 50,
        "re-asserting the held facing must not spend any TU"
    );
}

#[test]
fn set_facing_partial_turn_spends_exactly_the_afforded_steps() {
    let c = 7u8;
    let cost = TurnTu::new(c);
    let mut facing = Facing::new(Direction::North);
    let mut tu = Tu::new(2 * c);
    let before = *tu;

    let changed = *set_facing(&mut facing, &mut tu, Direction::South, &cost);

    assert!(
        changed,
        "a partly-affordable turn still turns (returns true)"
    );
    assert_eq!(
        *facing,
        Direction::North.rotated_toward(Direction::South, RingSteps::new(2)),
        "an under-affordable turn lands partway at the afforded short-way facing",
    );
    assert_eq!(
        *facing,
        Direction::East,
        "two short-way steps from North toward South land on East",
    );
    assert_eq!(
        before - *tu,
        2 * (*cost),
        "the charge is exactly the afforded steps (= 2) * the per-step cost",
    );
    assert_eq!(
        *tu, 0,
        "the 2-step charge drains the 2-step pool to exactly 0"
    );
}

#[test]
fn set_facing_with_pool_below_one_step_does_not_turn_or_charge() {
    let c = 5u8;
    let cost = TurnTu::new(c);
    let mut facing = Facing::new(Direction::North);
    let mut tu = Tu::new(c - 1);

    let changed = *set_facing(&mut facing, &mut tu, Direction::South, &cost);

    assert!(
        !changed,
        "a pool below one step's cost must report no turn (false)"
    );
    assert_eq!(
        *facing,
        Direction::North,
        "the facing is unchanged when not even one step is affordable",
    );
    assert_eq!(
        *tu,
        c - 1,
        "no charge is taken when not even one step is affordable",
    );

    let mut facing = Facing::new(Direction::North);
    let mut tu = Tu::new(0);
    let changed = *set_facing(&mut facing, &mut tu, Direction::South, &cost);
    assert!(!changed, "a broke ganger (Tu == 0) cannot turn");
    assert_eq!(
        *facing,
        Direction::North,
        "a broke ganger's facing is unchanged"
    );
    assert_eq!(*tu, 0, "a broke ganger is not charged");
}

#[test]
fn set_stance_charge_saturates_when_cost_exceeds_pool() {
    let cost = StanceChangeTu::new(200);
    let mut stance = Stance::new(StanceKind::Standing);
    let mut tu = Tu::new(5);

    let changed = *set_stance(&mut stance, &mut tu, StanceKind::Prone, &cost);

    assert!(
        changed,
        "the stance still changes even when TU cannot cover it"
    );
    assert_eq!(*stance, StanceKind::Prone, "the new stance is applied");
    assert_eq!(*tu, 0, "over-spending floors the pool at 0, never wraps");
}
