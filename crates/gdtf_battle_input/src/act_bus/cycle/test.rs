//! Tests for the authored cyclic orders (relocated from `cycle.rs`, GTW-201).

use gdtf_battle_sim::prelude::{Direction, StanceKind};

use crate::cycle::orders::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance};

/// AC8 — facing-cycle steps the 8 directions in the authored order and wraps
/// `NorthWest → North`. Asserting the exact next-of each pins the FACT.
#[test]
fn facing_cycle_steps_the_authored_compass_order_with_wrap() {
    assert_eq!(next_facing(Direction::North), Direction::NorthEast);
    assert_eq!(next_facing(Direction::NorthEast), Direction::East);
    assert_eq!(next_facing(Direction::East), Direction::SouthEast);
    assert_eq!(next_facing(Direction::SouthEast), Direction::South);
    assert_eq!(next_facing(Direction::South), Direction::SouthWest);
    assert_eq!(next_facing(Direction::SouthWest), Direction::West);
    assert_eq!(next_facing(Direction::West), Direction::NorthWest);
    // The wrap.
    assert_eq!(next_facing(Direction::NorthWest), Direction::North);
}

/// AC8 — stance-cycle steps the 3 postures and wraps `Prone → Standing`.
#[test]
fn stance_cycle_steps_the_posture_ladder_with_wrap() {
    assert_eq!(next_stance(StanceKind::Standing), StanceKind::Crouching);
    assert_eq!(next_stance(StanceKind::Crouching), StanceKind::Prone);
    // The wrap.
    assert_eq!(next_stance(StanceKind::Prone), StanceKind::Standing);
}

/// The authored orders enumerate the full closed sets (8 facings, 3 stances) —
/// so stepping is total and a full lap returns to the start.
#[test]
fn cycles_enumerate_the_full_sets_and_lap_back() {
    assert_eq!(FACING_CYCLE.len(), 8, "all 8 Directions are in the cycle");
    assert_eq!(STANCE_CYCLE.len(), 3, "all 3 StanceKinds are in the cycle");

    // A full lap of the facing cycle returns to North.
    let mut dir = Direction::North;
    for _ in 0..FACING_CYCLE.len() {
        dir = next_facing(dir);
    }
    assert_eq!(dir, Direction::North, "a full facing lap returns to North");

    // A full lap of the stance cycle returns to Standing.
    let mut stance = StanceKind::Standing;
    for _ in 0..STANCE_CYCLE.len() {
        stance = next_stance(stance);
    }
    assert_eq!(
        stance,
        StanceKind::Standing,
        "a full stance lap returns to Standing",
    );
}
