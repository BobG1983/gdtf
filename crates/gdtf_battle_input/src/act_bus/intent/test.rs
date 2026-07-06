//! Tests for the presenter level-step helper (relocated from `intent.rs`, GTW-201).

use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};

use crate::intent::level::{LevelStep, step_level};

/// AC6 — level-up steps toward the top and SATURATES at `MAX_LEVELS - 1`; it
/// never exceeds the grid's storey count.
#[test]
fn step_level_up_saturates_at_the_top_storey() {
    // From the ground floor, up moves one storey.
    assert_eq!(step_level(Level::new(0), LevelStep::Up), Level::new(1));
    // One below the top moves to the top.
    assert_eq!(
        step_level(Level::new(MAX_LEVELS - 2), LevelStep::Up),
        Level::new(MAX_LEVELS - 1),
    );
    // At the top, up saturates (stays at the top storey).
    assert_eq!(
        step_level(Level::new(MAX_LEVELS - 1), LevelStep::Up),
        Level::new(MAX_LEVELS - 1),
    );
}

/// AC6 — level-down steps toward the ground and FLOORS at `0`.
#[test]
fn step_level_down_floors_at_zero() {
    // From an upper storey, down moves one storey.
    assert_eq!(step_level(Level::new(3), LevelStep::Down), Level::new(2));
    // At the ground floor, down floors (stays at 0).
    assert_eq!(step_level(Level::new(0), LevelStep::Down), Level::new(0));
}
