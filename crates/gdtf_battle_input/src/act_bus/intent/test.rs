use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};

use crate::intent::level::{LevelStep, step_level};

#[test]
fn step_level_up_saturates_at_the_top_storey() {
    assert_eq!(step_level(Level::new(0), LevelStep::Up), Level::new(1));
    assert_eq!(
        step_level(Level::new(MAX_LEVELS - 2), LevelStep::Up),
        Level::new(MAX_LEVELS - 1),
    );
    assert_eq!(
        step_level(Level::new(MAX_LEVELS - 1), LevelStep::Up),
        Level::new(MAX_LEVELS - 1),
    );
}

#[test]
fn step_level_down_floors_at_zero() {
    assert_eq!(step_level(Level::new(3), LevelStep::Down), Level::new(2));
    assert_eq!(step_level(Level::new(0), LevelStep::Down), Level::new(0));
}
