use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelStep {
        Up,
        Down,
}

#[must_use]
pub fn step_level(current: Level, direction: LevelStep) -> Level {
    let top = MAX_LEVELS.saturating_sub(1);
    let raw = *current;
    let stepped = match direction {
        LevelStep::Up => {
            let up = raw.saturating_add(1);
            if up > top { top } else { up }
        }
        LevelStep::Down => raw.saturating_sub(1),
    };
    Level::new(stepped)
}
