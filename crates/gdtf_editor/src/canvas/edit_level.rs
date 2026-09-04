//! Current edit level and step helpers.

use bevy::prelude::*;
use gdtf_battle_sim::{level::GridSize, metric::Level};

/// Active storey the editor is painting on.
#[derive(Resource, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CurrentEditLevel(Level);

impl CurrentEditLevel {
    /// Ground storey (level 0).
    #[must_use]
    pub const fn ground() -> Self {
        Self(Level::new(0))
    }

    /// Step by `delta`, clamped to the map's level extent.
    #[must_use]
    pub fn stepped(self, delta: LevelStep, size: GridSize) -> Self {
        let current = i32::from(*self.0);
        let max = i32::from(*size.levels()).saturating_sub(1);
        let next = (current + delta.delta()).clamp(0, max);
        let storey = next as u8;
        Self(Level::new(storey))
    }

    /// Clamp to the map's level extent without stepping.
    #[must_use]
    pub fn clamped(self, size: GridSize) -> Self {
        self.stepped(LevelStep::none(), size)
    }

    /// Jump to `target`, clamped to the map's level extent.
    #[must_use]
    pub fn jumped(target: Level, size: GridSize) -> Self {
        Self(target).clamped(size)
    }

    /// Inner level value.
    #[must_use]
    pub const fn level(self) -> Level {
        self.0
    }
}

/// Signed step for moving between storeys.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelStep(i32);

impl LevelStep {
    /// One storey up.
    #[must_use]
    pub const fn up() -> Self {
        Self(1)
    }

    /// One storey down.
    #[must_use]
    pub const fn down() -> Self {
        Self(-1)
    }

    /// No step.
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    const fn delta(self) -> i32 {
        self.0
    }
}
