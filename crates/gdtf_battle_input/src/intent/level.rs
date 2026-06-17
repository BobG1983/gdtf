//! The presenter level-step helper drained by the level-cycle intents.

use gdtf_battle_sim::{Level, MAX_LEVELS};

/// Which way a level-step intent moves the
/// [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel).
///
/// A tiny domain enum so [`step_level`] reads `Up` / `Down` rather than a bare
/// sign — the two directions the level-cycle keys drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelStep {
    /// Toward a higher storey (saturating at `MAX_LEVELS - 1`).
    Up,
    /// Toward a lower storey (flooring at `0`).
    Down,
}

/// The [`Level`] after stepping `current` one storey in `direction`, clamped to the
/// valid `0..MAX_LEVELS` storey range.
///
/// `Up` saturates at the top storey (`MAX_LEVELS - 1`) — it never exceeds the grid's
/// storey count; `Down` floors at `0`. Saturating `u8` arithmetic, then a clamp to
/// the top storey, so the level can never wrap or escape the grid (the contract's
/// `0..MAX_LEVELS` clamp). Pure storey math; not a `const fn` because it deref-reads
/// the derived-`Deref` [`Level`] newtype, which is not a const operation.
#[must_use]
pub fn step_level(current: Level, direction: LevelStep) -> Level {
    // The top valid storey index. `MAX_LEVELS` is 8, so `MAX_LEVELS - 1` (= 7) is the
    // highest storey; `saturating_sub` guards the (impossible) `MAX_LEVELS == 0`.
    let top = MAX_LEVELS.saturating_sub(1);
    let raw = *current;
    let stepped = match direction {
        // Saturating add then clamp to the top storey: even if `raw` were already at
        // u8::MAX it could not wrap, and it can never exceed `top`.
        LevelStep::Up => {
            let up = raw.saturating_add(1);
            if up > top { top } else { up }
        }
        // Saturating sub floors at 0.
        LevelStep::Down => raw.saturating_sub(1),
    };
    Level::new(stepped)
}
