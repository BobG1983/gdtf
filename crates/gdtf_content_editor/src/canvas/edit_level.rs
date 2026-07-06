//! The GTW-500 C1 storey selector — [`CurrentEditLevel`] and [`LevelStep`], its
//! level-navigation delta vocabulary.

use bevy::prelude::*;
use gdtf_battle_sim::{level::GridSize, metric::Level};

/// The storey the canvas is currently editing (GTW-500 C1) — the x/y slice the egui viewport draws,
/// paints, and previews the hover ghost on.
///
/// A named newtype over the sim's [`Level`] storey index (no-bare-types: the edited storey is a
/// domain coordinate). PRIVATE inner, derived [`Deref`] to the wrapped [`Level`]; mutated through
/// [`stepped`](CurrentEditLevel::stepped) / [`clamped`](CurrentEditLevel::clamped), which keep the
/// value inside the prefab's storey range so the canvas can never read a slice past the drawable
/// volume. A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1), seeded to the ground storey.
#[derive(Resource, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CurrentEditLevel(Level);

impl CurrentEditLevel {
    /// The ground storey (`L0`) — the level the editor opens on (the GTW-423 canvas's old hardcoded
    /// plane, now the seed of the selector).
    #[must_use]
    pub const fn ground() -> Self {
        Self(Level::new(0))
    }

    /// This level stepped by `delta` storeys, CLAMPED to the prefab's `[0, levels-1]` range so the
    /// result is always inside the drawable volume (C1). A step that would leave the range saturates
    /// at the nearest end.
    #[must_use]
    pub fn stepped(self, delta: LevelStep, size: GridSize) -> Self {
        let current = i32::from(*self.0);
        let max = i32::from(*size.levels()).saturating_sub(1);
        let next = (current + delta.delta()).clamp(0, max);
        // `next` is clamped into `[0, max]` where `max < levels <= MAX_LEVELS` (a `u8`), so the
        // `u8` conversion is always in range — no panic, no truncation in practice.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "next is clamped to [0, levels-1] with levels <= MAX_LEVELS (u8), so it fits \
                      a u8 without wrap or sign-flip"
        )]
        let storey = next as u8;
        Self(Level::new(storey))
    }

    /// This level CLAMPED to the prefab's `[0, levels-1]` range — used when the grid shrinks below
    /// the current storey (a size change must never leave the selector pointing past the new
    /// volume).
    #[must_use]
    pub fn clamped(self, size: GridSize) -> Self {
        self.stepped(LevelStep::none(), size)
    }

    /// The wrapped storey index — the [`Level`] the canvas render / paint / ghost read.
    #[must_use]
    pub const fn level(self) -> Level {
        self.0
    }
}

/// A signed level-navigation step in storeys (no-bare-types: a step is a domain delta). `+1` steps
/// up one storey, `-1` down; `0` is the identity used for a re-clamp.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelStep(i32);

impl LevelStep {
    /// Step UP one storey (toward the ceiling).
    #[must_use]
    pub const fn up() -> Self {
        Self(1)
    }

    /// Step DOWN one storey (toward the ground).
    #[must_use]
    pub const fn down() -> Self {
        Self(-1)
    }

    /// No step — the identity used to re-clamp the current level after a grid shrink.
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    /// The signed storey delta.
    const fn delta(self) -> i32 {
        self.0
    }
}
