//! What the posture cost queries quote, and what the stance and turn verbs charge for it.

use crate::{
    ganger::{Direction, Facing, RingSteps, Stance, StanceKind, Tu},
    posture::{
        FacingRefusal, StanceRefusal, afforded_turn_steps, afforded_turn_tu_cost, can_set_facing,
        can_set_stance, facing_refusal, set_aiming_tu_cost, set_facing, set_stance, stance_refusal,
        stance_tu_cost, turn_tu_cost,
    },
    tuning::{StanceChangeTu, TurnTu},
};

#[test]
fn set_aiming_tu_cost_quotes_zero_because_aiming_is_free() {
    assert_eq!(
        *set_aiming_tu_cost(),
        0,
        "aiming is free, so the quote must be zero TU",
    );
}

#[test]
fn set_stance_charges_exactly_stance_tu_cost() {
    let cost = StanceChangeTu::new(11);
    let mut stance = Stance::new(StanceKind::Standing);
    let mut tu = Tu::new(60);
    let before = *tu;

    let quoted = stance_tu_cost(&cost);
    set_stance(&mut stance, &mut tu, StanceKind::Prone, &cost);

    assert_eq!(
        before - *tu,
        *quoted,
        "set_stance must charge exactly what stance_tu_cost quoted",
    );
    assert_eq!(
        *quoted, *cost,
        "stance_tu_cost must read the injected StanceChangeTu leaf",
    );
}

#[test]
fn can_set_stance_answers_change_and_affordability() {
    let cost = StanceChangeTu::new(11);
    let standing = Stance::new(StanceKind::Standing);

    assert!(
        *can_set_stance(&standing, StanceKind::Prone, &Tu::new(*cost), &cost),
        "a different stance with a pool that covers the quote is legal",
    );
    assert!(
        !*can_set_stance(&standing, StanceKind::Standing, &Tu::new(60), &cost),
        "re-asserting the held stance is not a legal change",
    );
    assert!(
        !*can_set_stance(
            &standing,
            StanceKind::Prone,
            &Tu::new(cost.saturating_sub(1)),
            &cost
        ),
        "a pool below the quote cannot pay for a stance change",
    );
}

#[test]
fn stance_refusal_names_the_rule_the_change_breaks() {
    let cost = StanceChangeTu::new(11);
    let standing = Stance::new(StanceKind::Standing);

    assert_eq!(
        stance_refusal(&standing, StanceKind::Prone, &Tu::new(*cost), &cost),
        None,
        "a different stance with a pool that covers the quote breaks no rule",
    );
    assert_eq!(
        stance_refusal(&standing, StanceKind::Standing, &Tu::new(60), &cost),
        Some(StanceRefusal::AlreadyHeld),
        "re-asserting the held stance is refused for the stance held, not for the price",
    );
    assert_eq!(
        stance_refusal(
            &standing,
            StanceKind::Prone,
            &Tu::new(cost.saturating_sub(1)),
            &cost,
        ),
        Some(StanceRefusal::Unaffordable),
        "a pool below the quote is refused as unaffordable, not as a stance already held",
    );
}

#[test]
fn turn_tu_cost_is_per_step_and_set_facing_charges_the_afforded_quote() {
    let cost = TurnTu::new(6);
    let (from, to) = (Direction::North, Direction::South);
    let steps = from.steps_to(to);

    assert_eq!(
        *turn_tu_cost(steps, &cost),
        (*steps).saturating_mul(*cost),
        "turn_tu_cost must be per-step TurnTu times the ring steps",
    );

    let mut facing = Facing::new(from);
    let mut tu = Tu::new(2 * *cost);
    let quoted = afforded_turn_tu_cost(from, to, &tu, &cost);
    let before = *tu;

    set_facing(&mut facing, &mut tu, to, &cost);

    assert_eq!(
        before - *tu,
        *quoted,
        "set_facing must charge exactly what afforded_turn_tu_cost quoted",
    );
    assert_eq!(
        afforded_turn_steps(from, to, &Tu::new(before), &cost),
        RingSteps::new(2),
        "a two-step pool affords exactly two of the four short-way steps",
    );
}

#[test]
fn set_facing_full_turn_charges_the_unclamped_quote() {
    let cost = TurnTu::new(3);
    let (from, to) = (Direction::North, Direction::East);
    let steps = from.steps_to(to);

    let mut facing = Facing::new(from);
    let mut tu = Tu::new(90);
    let before = *tu;

    set_facing(&mut facing, &mut tu, to, &cost);

    assert_eq!(
        before - *tu,
        *turn_tu_cost(steps, &cost),
        "an affordable turn charges the full turn_tu_cost for its ring steps",
    );
    assert_eq!(*facing, to, "an affordable turn lands on the request");
}

#[test]
fn can_set_facing_answers_turn_and_affordability() {
    let cost = TurnTu::new(6);
    let (from, to) = (Direction::North, Direction::South);

    assert!(
        *can_set_facing(from, to, &Tu::new(*cost), &cost),
        "one affordable step is enough to call the turn legal",
    );
    assert!(
        !*can_set_facing(from, from, &Tu::new(90), &cost),
        "re-asserting the held facing is not a legal turn",
    );
    assert!(
        !*can_set_facing(from, to, &Tu::new(cost.saturating_sub(1)), &cost),
        "a pool below one step's cost cannot pay for any turn",
    );
}

#[test]
fn facing_refusal_names_the_rule_the_turn_breaks() {
    let cost = TurnTu::new(6);
    let (from, to) = (Direction::North, Direction::South);

    assert_eq!(
        facing_refusal(from, to, &Tu::new(*cost), &cost),
        None,
        "a turn of at least one step the pool can pay for breaks no rule",
    );
    assert_eq!(
        facing_refusal(from, from, &Tu::new(90), &cost),
        Some(FacingRefusal::AlreadyFacing),
        "a turn of zero steps is refused for the direction already faced, not for the price",
    );
    assert_eq!(
        facing_refusal(from, to, &Tu::new(cost.saturating_sub(1)), &cost),
        Some(FacingRefusal::Unaffordable),
        "a pool below one ring step is refused as unaffordable, not as a facing already held",
    );
}
