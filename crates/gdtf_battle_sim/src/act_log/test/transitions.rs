//! The prior-value maps: a FIRST observation seeds silently, a later CHANGE reports a
//! transition, and an unchanged re-observation reports nothing (GTW-727 C11).

use bevy::prelude::Entity;

use crate::{
    act_log::{ActLog, PoseFacts, SuppressedNow},
    ganger::{Aiming, Direction, Facing, LifeState, Stance, StanceKind},
};

/// A pose with the given facing, everything else at rest.
fn pose(direction: Direction) -> PoseFacts {
    PoseFacts::new(
        Facing::new(direction),
        Stance::new(StanceKind::Standing),
        Aiming::new(false),
        SuppressedNow::new(false),
    )
}

/// The seed-silently rule, which is what stops a roster spawn from flooding the log:
/// situation setup writes facing, stance and aiming on every ganger at once, so a
/// recorder that fired on a first observation would record one entry per ganger before
/// anybody had acted.
#[test]
fn a_first_pose_observation_seeds_without_reporting_a_transition() {
    let mut log = ActLog::default();
    let ganger = Entity::from_raw_u32(1).unwrap_or(Entity::PLACEHOLDER);

    assert!(
        !log.note_pose(ganger, pose(Direction::North)),
        "a first observation must SEED the prior-value map and report no transition",
    );
    assert!(
        !log.note_pose(ganger, pose(Direction::North)),
        "re-observing the SAME pose is not a transition",
    );
    assert!(
        log.note_pose(ganger, pose(Direction::South)),
        "a changed pose IS a transition",
    );
    assert!(
        !log.note_pose(ganger, pose(Direction::South)),
        "the map is updated by the transition, so the new value is now the baseline",
    );
}

/// The life-state map returns the state the ganger LEFT — the `from` half of a genuine
/// transition, which no post-act snapshot could produce.
#[test]
fn a_life_transition_reports_the_state_that_was_left() {
    let mut log = ActLog::default();
    let ganger = Entity::from_raw_u32(2).unwrap_or(Entity::PLACEHOLDER);

    assert_eq!(
        log.note_life(ganger, LifeState::Alive),
        None,
        "a first observation seeds silently",
    );
    assert_eq!(
        log.note_life(ganger, LifeState::Alive),
        None,
        "an unchanged life state is not a transition",
    );
    assert_eq!(
        log.note_life(ganger, LifeState::Downed),
        Some(LifeState::Alive),
        "the transition reports the state LEFT, so `from → to` is expressible",
    );
    assert_eq!(
        log.note_life(ganger, LifeState::Dead),
        Some(LifeState::Downed),
        "a second transition reports the intermediate state, not the original one",
    );
}

/// Prior values are per-entity: one ganger's change must not mask or trigger another's.
#[test]
fn prior_values_are_tracked_per_entity() {
    let mut log = ActLog::default();
    let first = Entity::from_raw_u32(3).unwrap_or(Entity::PLACEHOLDER);
    let second = Entity::from_raw_u32(4).unwrap_or(Entity::PLACEHOLDER);

    assert!(!log.note_pose(first, pose(Direction::North)));
    assert!(!log.note_pose(second, pose(Direction::North)));
    assert!(log.note_pose(first, pose(Direction::East)));
    assert!(
        !log.note_pose(second, pose(Direction::North)),
        "the second ganger did not move, so its own baseline is untouched",
    );
}
