//! Active-level step direction and clamp.

use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};

/// Direction to step the active level view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelStep {
    /// One storey up.
    Up,
    /// One storey down.
    Down,
}

/// Step `current` by one storey, clamped to `[0, MAX_LEVELS)`.
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
